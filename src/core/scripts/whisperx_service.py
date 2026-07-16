#!/usr/bin/env python3
"""
WhisperX Persistent Service
Keeps WhisperX model loaded in memory and serves transcription requests via HTTP.
This eliminates model loading time on each request.

Usage:
    python whisperx_service.py --port 8001 --device cuda --compute_type float16
"""

import os
import sys
import argparse
import json
import tempfile
from pathlib import Path
from flask import Flask, request, jsonify
import whisperx
import logging

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

app = Flask(__name__)

# Global model instance (loaded once at startup)
WHISPERX_MODEL = None
MODEL_CONFIG = {}


def load_model(model_name, device, compute_type, language):
    """Load WhisperX model into memory"""
    logger.info(f"Loading WhisperX model: {model_name} on {device} with {compute_type}")
    
    model = whisperx.load_model(
        model_name,
        device=device,
        compute_type=compute_type,
        language=language
    )
    
    logger.info(f"✅ WhisperX model loaded successfully")
    return model


@app.route('/health', methods=['GET'])
def health_check():
    """Health check endpoint"""
    return jsonify({
        'status': 'healthy',
        'model_loaded': WHISPERX_MODEL is not None,
        'config': MODEL_CONFIG
    })


@app.route('/transcribe', methods=['POST'])
def transcribe():
    """
    Transcribe audio file
    
    Request:
        - audio_file: path to audio file
        - uuid: unique identifier for this transcription
        - output_dir: directory to save output files
        - output_format: format(s) to generate (txt, vtt, srt, json, tsv, all)
    
    Response:
        - success: boolean
        - transcription: full text
        - segments: list of segments with timestamps
        - output_files: paths to generated files
    """
    try:
        data = request.get_json()
        
        audio_file = data.get('audio_file')
        uuid = data.get('uuid', 'unknown')
        output_dir = data.get('output_dir', '.')
        output_format = data.get('output_format', 'all')
        
        if not audio_file:
            return jsonify({'success': False, 'error': 'audio_file is required'}), 400
        
        audio_path = Path(audio_file)
        if not audio_path.exists():
            return jsonify({'success': False, 'error': f'Audio file not found: {audio_file}'}), 404
        
        logger.info(f"Transcribing {audio_file} (uuid: {uuid})")
        
        # Load audio
        audio = whisperx.load_audio(str(audio_path))
        
        # Transcribe
        batch_size = 16 if MODEL_CONFIG['device'] == 'cuda' else 8
        result = WHISPERX_MODEL.transcribe(
            audio,
            batch_size=batch_size,
            language=MODEL_CONFIG['language']
        )
        
        # Align whisper output to get word-level timestamps
        language = result.get("language", MODEL_CONFIG['language'])
        
        # Preserve original word texts before alignment (alignment can corrupt them)
        original_words = {}
        for seg in result.get("segments", []):
            for w in seg.get("words", []):
                word_text = w.get("word", "")
                start = w.get("start", None)
                if start is not None:
                    original_words[(round(start, 3), word_text.strip().lower())] = word_text
        
        # Split segment text into words as fallback for word recovery
        original_segment_words = []
        for seg in result.get("segments", []):
            original_segment_words.append(seg.get("text", "").strip().split())
        
        try:
            model_a, metadata = whisperx.load_align_model(language_code=language, device=MODEL_CONFIG['device'])
            result = whisperx.align(result["segments"], model_a, metadata, audio, MODEL_CONFIG['device'], return_char_alignments=False)
            del model_a
            
            # Restore original word texts (alignment only provides timing/scores)
            for seg_idx, seg in enumerate(result.get("segments", [])):
                seg_words = original_segment_words[seg_idx] if seg_idx < len(original_segment_words) else []
                aligned_words = seg.get("words", [])
                for w_idx, w in enumerate(aligned_words):
                    if w_idx < len(seg_words):
                        w["word"] = seg_words[w_idx]
        except Exception as align_err:
            logger.warning(f"Alignment failed, proceeding without word-level timestamps: {align_err}")
        
        # Extract segments and full text
        segments = result.get("segments", [])
        full_text = " ".join([seg.get("text", "").strip() for seg in segments])
        
        logger.info(f"Transcription complete: {len(segments)} segments, {len(full_text)} characters")
        
        # Generate output files
        output_path = Path(output_dir)
        output_path.mkdir(parents=True, exist_ok=True)
        
        base_name = audio_path.stem
        output_base = output_path / base_name
        
        output_files = {}
        formats = ['txt', 'vtt', 'srt', 'json', 'tsv'] if output_format == 'all' else [output_format]
        
        for fmt in formats:
            if fmt == 'txt':
                file_path = write_txt(segments, output_base)
                output_files['txt'] = str(file_path)
            elif fmt == 'vtt':
                file_path = write_vtt(segments, output_base)
                output_files['vtt'] = str(file_path)
            elif fmt == 'srt':
                file_path = write_srt(segments, output_base)
                output_files['srt'] = str(file_path)
            elif fmt == 'json':
                file_path = write_json_file(segments, output_base)
                output_files['json'] = str(file_path)
            elif fmt == 'tsv':
                file_path = write_tsv(segments, output_base)
                output_files['tsv'] = str(file_path)
        
        return jsonify({
            'success': True,
            'transcription': full_text,
            'segments': segments,
            'output_files': output_files,
            'language': result.get('language', MODEL_CONFIG['language']),
            'num_segments': len(segments)
        })
    
    except Exception as e:
        logger.error(f"Transcription error: {str(e)}", exc_info=True)
        return jsonify({'success': False, 'error': str(e)}), 500


def write_txt(segments, output_base):
    """Write plain text transcription"""
    file_path = f"{output_base}.txt"
    with open(file_path, "w", encoding="utf-8") as f:
        for segment in segments:
            f.write(segment.get("text", "").strip() + " ")
    return file_path


def write_vtt(segments, output_base):
    """Write WebVTT format"""
    file_path = f"{output_base}.vtt"
    with open(file_path, "w", encoding="utf-8") as f:
        f.write("WEBVTT\n\n")
        for segment in segments:
            start = format_timestamp_vtt(segment.get("start", 0))
            end = format_timestamp_vtt(segment.get("end", 0))
            f.write(f"{start} --> {end}\n")
            f.write(f"{segment.get('text', '').strip()}\n\n")
    return file_path


def write_srt(segments, output_base):
    """Write SubRip (SRT) format"""
    file_path = f"{output_base}.srt"
    with open(file_path, "w", encoding="utf-8") as f:
        for i, segment in enumerate(segments, 1):
            start = format_timestamp_srt(segment.get("start", 0))
            end = format_timestamp_srt(segment.get("end", 0))
            f.write(f"{i}\n")
            f.write(f"{start} --> {end}\n")
            f.write(f"{segment.get('text', '').strip()}\n\n")
    return file_path


def write_json_file(segments, output_base):
    """Write JSON format with timestamps in the standard format"""
    file_path = f"{output_base}.json"
    segments_data = []
    total_segments = len(segments)
    
    for i, segment in enumerate(segments):
        text = segment.get("text", "").strip()
        start_time = segment.get("start", 0)
        end_time = segment.get("end", 0)
        words_data = []
        
        # Build words array from segment words if available
        raw_words = segment.get("words", [])
        for word_info in raw_words:
            word_text = word_info.get("word", "")
            word_score = int(word_info.get("score", 0) * 100) if isinstance(word_info.get("score", 0), float) and word_info.get("score", 0) <= 1.0 else int(word_info.get("score", 0))
            words_data.append({
                "text": word_text,
                "score": word_score,
                "start_time": round(word_info.get("start", 0), 2),
                "end_time": round(word_info.get("end", 0), 2)
            })
        
        # Calculate alternative score as sum of word scores
        alt_score = sum(w.get("score", 0) for w in words_data)
        
        is_last = (i == total_segments - 1)
        
        segment_entry = {
            "alternatives": [
                {
                    "text": text,
                    "words": words_data,
                    "score": alt_score,
                    "lm": "whisperx:large-v3"
                }
            ],
            "segment_index": i,
            "last_segment": is_last,
            "final_result": is_last,
            "start_time": round(start_time, 2),
            "end_time": round(end_time, 2),
            "result_status": "RECOGNIZED"
        }
        segments_data.append(segment_entry)
    
    # Add final empty segment marker
    if segments_data:
        last = segments_data[-1]
        final_marker = {
            "alternatives": [
                {
                    "text": "",
                    "words": [],
                    "score": 0,
                    "lm": "whisperx:large-v3"
                }
            ],
            "segment_index": total_segments,
            "last_segment": True,
            "final_result": True,
            "start_time": last["start_time"],
            "end_time": last["end_time"],
            "result_status": "RECOGNIZED"
        }
        segments_data.append(final_marker)
    
    with open(file_path, "w", encoding="utf-8") as f:
        json.dump(segments_data, f, ensure_ascii=False, indent=2)
    return file_path


def write_tsv(segments, output_base):
    """Write TSV format"""
    file_path = f"{output_base}.tsv"
    with open(file_path, "w", encoding="utf-8") as f:
        f.write("start\tend\ttext\n")
        for segment in segments:
            start_ms = int(segment.get("start", 0) * 1000)
            end_ms = int(segment.get("end", 0) * 1000)
            text = segment.get("text", "").strip().replace("\t", " ")
            f.write(f"{start_ms}\t{end_ms}\t{text}\n")
    return file_path


def format_timestamp_vtt(seconds):
    """Format timestamp for VTT (HH:MM:SS.mmm)"""
    hours = int(seconds // 3600)
    minutes = int((seconds % 3600) // 60)
    secs = seconds % 60
    return f"{hours:02d}:{minutes:02d}:{secs:06.3f}"


def format_timestamp_srt(seconds):
    """Format timestamp for SRT (HH:MM:SS,mmm)"""
    hours = int(seconds // 3600)
    minutes = int((seconds % 3600) // 60)
    secs = seconds % 60
    return f"{hours:02d}:{minutes:02d}:{secs:06.3f}".replace('.', ',')


def main():
    global WHISPERX_MODEL, MODEL_CONFIG
    
    parser = argparse.ArgumentParser(description='WhisperX Persistent Service')
    parser.add_argument('--port', type=int, default=8001, help='Port to listen on')
    parser.add_argument('--host', default='127.0.0.1', help='Host to bind to')
    parser.add_argument('--model', default='large-v3', help='WhisperX model name')
    parser.add_argument('--device', default='cuda', choices=['cpu', 'cuda'], help='Device to use')
    parser.add_argument('--compute_type', default='float16', choices=['int8', 'float16', 'float32'], help='Compute type')
    parser.add_argument('--language', default='pt', help='Language code')
    
    args = parser.parse_args()
    
    # Store config
    MODEL_CONFIG = {
        'model': args.model,
        'device': args.device,
        'compute_type': args.compute_type,
        'language': args.language
    }
    
    # Load model at startup
    logger.info("=" * 60)
    logger.info("Starting WhisperX Persistent Service")
    logger.info("=" * 60)
    
    try:
        WHISPERX_MODEL = load_model(args.model, args.device, args.compute_type, args.language)
    except Exception as e:
        logger.error(f"Failed to load WhisperX model: {e}")
        sys.exit(1)
    
    logger.info(f"Starting HTTP server on {args.host}:{args.port}")
    logger.info("=" * 60)
    
    # Run Flask app
    app.run(host=args.host, port=args.port, threaded=True)


if __name__ == '__main__':
    main()
