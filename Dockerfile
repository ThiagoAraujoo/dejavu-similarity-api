# Build stage - Use Ubuntu 22.04 to match deployment server
FROM ubuntu:22.04 AS builder

WORKDIR /app

# Install Rust and build dependencies
RUN apt-get update && \
    apt-get install -y curl build-essential pkg-config libssl-dev ca-certificates && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
    rm -rf /var/lib/apt/lists/*

ENV PATH="/root/.cargo/bin:${PATH}"

# Copy manifests first for better caching
COPY Cargo.toml ./

# Generate Cargo.lock if it doesn't exist
RUN cargo fetch

# Copy source code
COPY src ./src

# Build for release
RUN cargo build --release

# Runtime stage - Use Ubuntu 22.04 to match deployment server
FROM ubuntu:22.04

# Install runtime dependencies including FFmpeg for noise removal and Python3 for similarity scripts
RUN apt-get update && \
    apt-get install -y ca-certificates libssl3 ffmpeg python3 python3-pip && \
    pip3 install --no-cache-dir \
        sentence-transformers \
        faster-whisper \
        torch \
        transformers \
        whisperx && \
    rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/dejavu-similarity-api /usr/local/bin/dejavu-similarity-api

# Copy all Python scripts to fixed location
COPY src/core/scripts/semantic_similarity_detector.py /app/core/scripts/
COPY src/core/scripts/faster_whisper_cli.py /app/core/scripts/
COPY src/core/scripts/distil_whisper_cli.py /app/core/scripts/
COPY src/core/scripts/whisperx_cli.py /app/core/scripts/

# Copy .env if needed (or use environment variables)
COPY .env /app/.env

WORKDIR /app

EXPOSE ${APP_PORT}

CMD ["dejavu-similarity-api"]