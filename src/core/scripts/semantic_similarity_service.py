#!/usr/bin/env python3
"""
Semantic Similarity Persistent Service
Keeps the SentenceTransformer model loaded in memory and serves similarity
requests via HTTP, eliminating model loading time on each request.

Usage:
    python semantic_similarity_service.py --port 8002 --host 127.0.0.1
"""

import argparse
import logging
import os
import sys
import time

from flask import Flask, request, jsonify

# Import detection logic from the existing CLI script.
# The model is loaded at module import time and reused for every request.
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from semantic_similarity_detector import detect_advertisement, MODEL

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

app = Flask(__name__)


@app.route('/health', methods=['GET'])
def health_check():
    """Health check endpoint."""
    return jsonify({
        'status': 'healthy',
        'model_loaded': MODEL is not None,
    })


@app.route('/detect', methods=['POST'])
def detect():
    """
    Detect if an advertisement appears in a program transcription.

    Request:
        - program_text: full program transcription
        - ad_text: advertisement transcription to search for

    Response:
        - match_found: bool
        - score: int (0-100)
        - matched_snippet: str
        - overall_similarity: float
        - chunk_similarity: float
        - ad_keywords: list[str]
        - matched_keywords: list[str]
    """
    start_time = time.time()
    data = request.get_json() or {}

    program_text = data.get('program_text', '')
    ad_text = data.get('ad_text', '')

    if not isinstance(program_text, str) or not isinstance(ad_text, str):
        return jsonify({'error': 'program_text and ad_text must be strings'}), 400

    try:
        result = detect_advertisement(program_text, ad_text)
        elapsed = time.time() - start_time
        logger.info(
            "[PERF] Similarity detection completed in %.2fms: match=%s, score=%s",
            elapsed * 1000,
            result.get('match_found'),
            result.get('score'),
        )
        return jsonify(result)
    except Exception as e:
        logger.error("Similarity detection failed: %s", e, exc_info=True)
        return jsonify({'error': str(e)}), 500


def main():
    parser = argparse.ArgumentParser(
        description='Semantic Similarity Persistent Service'
    )
    parser.add_argument('--host', default='127.0.0.1', help='Host to bind to')
    parser.add_argument('--port', type=int, default=8002, help='Port to listen on')
    args = parser.parse_args()

    logger.info("=" * 60)
    logger.info("Starting Semantic Similarity Persistent Service")
    logger.info("Model loaded: %s", MODEL is not None)
    logger.info("=" * 60)

    logger.info("Starting HTTP server on %s:%s", args.host, args.port)
    app.run(host=args.host, port=args.port, threaded=True)


if __name__ == '__main__':
    main()
