# Dejavu Similarity API

A high-performance semantic similarity API built with Rust. It detects whether an advertisement transcription appears inside a program transcription using a persistent SentenceTransformer service.

## Features

- **Persistent Similarity Model**: The SentenceTransformer model is loaded once in a Python HTTP service and reused for every request.
- **WebSocket Interface**: Real-time similarity detection via WebSocket.
- **CLI Fallback**: Optional fallback to a Python CLI script if the HTTP service is unavailable.
- **HTTP/CLI Flexibility**: Use `SIMILARITY_SERVICE_URL` to point to the service, or set `SIMILARITY_SERVICE_URL=cli` to force CLI mode.

## Prerequisites

- Rust 1.70+ (for building the API)
- Python 3.8+ with `sentence-transformers`, `torch`, `transformers`, and `flask` (for the similarity service)

## Installation

### 1. Install Python Dependencies

```bash
pip install sentence-transformers torch transformers flask
```

### 2. Clone and Build

```bash
git clone <repository-url>
cd dejavu-similarity-api
cargo build --release
```

### 3. Configure Environment

```bash
cp .env.example .env
```

Edit `.env` and set at least:

```env
WEBSOCKET_AUTH_TOKEN=your-secure-token-here
SIMILARITY_SERVICE_URL=http://127.0.0.1:8002
```

### 4. Run the Similarity Service

```bash
python3 src/core/scripts/semantic_similarity_service.py --port 8002
```

### 5. Run the API

```bash
cargo run --release
```

The API will start on `http://localhost:3000` (or the port specified in your `.env`).

## API Usage

### Similarity Detection WebSocket

**Endpoint**: `ws://localhost:3000/similarity?token=YOUR_TOKEN`

**Request**:

```json
{
  "uuid": "unique-request-id",
  "programming_id": 1,
  "programming_transcription": "full program text",
  "advertisement_id": 2,
  "advertisement_transcription": "advertisement text"
}
```

**Response**:

```json
{
  "uuid": "unique-request-id",
  "programming_id": 1,
  "programming_transcription": "full program text",
  "advertisement_id": 2,
  "advertisement_transcription": "advertisement text",
  "match_found": true,
  "score": 85,
  "error": null
}
```

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `SIMILARITY_SERVICE_URL` | `http://127.0.0.1:8002` | URL of the persistent similarity service; set to `cli` to force CLI fallback |
| `SEMANTIC_DETECTOR_PATH` | `/app/core/scripts/semantic_similarity_detector.py` | Path to the CLI fallback Python script |
| `SIMILARITY_THRESHOLD` | `45.0` | Minimum score to consider a match |
| `SIMILARITY_WINDOW_SIZE` | `50` | Chunk window size for the detector |
| `SIMILARITY_WINDOW_OVERLAP` | `25` | Chunk overlap for the detector |
| `MAX_CONCURRENT_TASKS` | CPU count | Concurrent similarity requests on the WebSocket |
| `TASK_TIMEOUT_SECONDS` | `30` | Timeout per similarity request |
| `WEBSOCKET_AUTH_TOKEN` | - | Authentication token for API access |
| `APP_PORT` | `3000` | Server port |

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

The Docker Compose setup starts both the Rust API and the persistent Python similarity service.

## License

[Your License Here]

## Contributing

Thiago Silveira de Araujo