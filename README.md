# Dejavu Similarity API

A high-performance audio similarity API built with Rust and faster-whisper, optimized for Portuguese language similarity.

## Features

- **4x Faster Similarity**: Uses faster-whisper implementation for significantly improved performance
- **Portuguese Optimized**: Configured specifically for Portuguese language similarity
- **Multiple Output Formats**: Returns transcriptions in VTT, SRT, JSON, and TSV formats
- **Real-time Updates**: WebSocket support for live similarity status updates
- **Noise Removal**: Optional audio preprocessing for better accuracy
- **RESTful API**: Simple HTTP endpoints for file upload and processing

## Prerequisites

- Rust 1.70+ (for building the API)
- Python 3.8+ (for faster-whisper)
- PostgreSQL (for data persistence)

## Installation

### 1. Install faster-whisper

```bash
pip install faster-whisper
```

### 2. Clone and Build

```bash
git clone <repository-url>
cd dejavu-similarity-api
cargo build --release
```

### 3. Configure Environment

Copy the example environment file and configure it:

```bash
cp .env.example .env
```

Edit `.env` and set your configuration:

```env
# Similarity (faster-whisper configuration)
WHISPER_PATH=faster-whisper
WHISPER_MODEL=large-v3
WHISPER_LANGUAGE=pt

# Authentication
WEBSOCKET_AUTH_TOKEN=your-secure-token-here
```

### 4. Run the API

```bash
cargo run --release
```

The API will start on `http://localhost:8080` (or the port specified in your `.env`).

## API Usage

### Upload Audio for Similarity

**Endpoint**: `POST /api/similarity/upload`

**Request** (multipart/form-data):
- `file`: Audio file (mp3, wav, m4a, ogg, flac)
- `token`: Authentication token
- `apply_noise_removal`: Optional, "true" or "false" (default: true)

**Response**:
```json
{
  "job_id": "019abc12-3def-4567-8901-234567890abc",
  "status": "processing",
  "message": "Similarity job started. Connect to WebSocket for updates."
}
```

### WebSocket Status Updates

**Endpoint**: `ws://localhost:8080/ws/similarity/status?token=YOUR_TOKEN`

**Status Update Format**:
```json
{
  "job_id": "019abc12-3def-4567-8901-234567890abc",
  "status": "completed",
  "progress": 100.0,
  "message": "Similarity completed successfully",
  "result": {
    "uuid": "019abc12-3def-4567-8901-234567890abc",
    "similarity": "Complete similarity text in Portuguese",
    "vtt": "WEBVTT\n\n00:00:00.000 --> 00:00:05.000\nTexto da transcrição...",
    "srt": "1\n00:00:00,000 --> 00:00:05,000\nTexto da transcrição...",
    "json_file": "[{\"start\": 0.0, \"end\": 5.0, \"text\": \"Texto...\"}]",
    "tsv": "start\tend\ttext\n0\t5000\tTexto da transcrição...",
    "duration_seconds": 120.5,
    "language": "pt"
  },
  "error": null
}
```

## Output Formats

The API returns transcriptions in multiple formats:

- **similarity**: Plain text similarity
- **vtt**: WebVTT format with timestamps (for web video players)
- **srt**: SubRip format with timestamps (for video subtitles)
- **json_file**: JSON array with word/segment-level timestamps
- **tsv**: Tab-separated values with start/end times and text

## Performance

With faster-whisper and the `large-v3` model:
- **Speed**: ~4x faster than standard Whisper
- **Accuracy**: Excellent for Portuguese language
- **Model Size**: ~3GB (large-v3)

### Model Options

You can configure different models via `WHISPER_MODEL` environment variable:

- `tiny`: Fastest, lowest accuracy (~75MB)
- `base`: Fast, decent accuracy (~150MB)
- `small`: Balanced (~500MB)
- `medium`: Good accuracy (~1.5GB)
- `large-v3`: Best accuracy, slower (~3GB) - **Recommended**

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `WHISPER_PATH` | `faster-whisper` | Command to run similarity |
| `WHISPER_MODEL` | `large-v3` | Whisper model to use |
| `WHISPER_LANGUAGE` | `pt` | Language code (pt = Portuguese) |
| `WEBSOCKET_AUTH_TOKEN` | - | Authentication token for API access |
| `HOST` | `0.0.0.0` | Server host |
| `PORT` | `8080` | Server port |

## Development

### Build for Development 

```bash
cargo build
cargo run
```

### Run Tests

```bash
cargo test
```

### Docker Deployment

```bash
docker-compose up -d
```

## Troubleshooting

### faster-whisper not found

Ensure faster-whisper is installed and in your PATH:
```bash
which faster-whisper
pip install --upgrade faster-whisper
```

### Model download issues

On first run, faster-whisper will download the model. Ensure you have:
- Internet connection
- Sufficient disk space (~3GB for large-v3)
- Write permissions in the model cache directory

### Performance issues

- Use a smaller model (`medium` or `small`) for faster processing
- Ensure you're using faster-whisper, not standard whisper
- Check CPU/GPU availability (faster-whisper supports CUDA)

## License

[Your License Here]

## Contributing

[Contributing Guidelines Here]