#!/usr/bin/env python3
"""
Persistent FastAPI service for semantic similarity detection.
Keeps the SentenceTransformer model in memory to avoid loading overhead on every request.
"""

import os
import sys
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel
import uvicorn

# Import detection logic from the existing script
from semantic_similarity_detector import detect_advertisement, ensure_model_loaded

app = FastAPI(title="Dejavu Similarity Detection Service")


@app.on_event("startup")
async def startup_event():
    """Load the model into memory on service startup."""
    print("Loading SentenceTransformer model into memory...", file=sys.stderr)
    ensure_model_loaded()
    print("✅ Model loaded and ready for inference", file=sys.stderr)


class SimilarityRequest(BaseModel):
    """Request payload for similarity detection."""
    program_text: str
    ad_text: str


class SimilarityResponse(BaseModel):
    """Response payload for similarity detection."""
    match_found: bool
    score: int
    matched_snippet: str
    overall_similarity: float
    chunk_similarity: float
    ad_keywords: list[str]
    matched_keywords: list[str]


@app.post("/detect", response_model=SimilarityResponse)
async def detect(request: SimilarityRequest) -> dict:
    """
    Detect if an advertisement appears in a program transcription.
    
    Args:
        request: Contains program_text and ad_text
        
    Returns:
        Detection result with match status, score, and matched snippet
    """
    try:
        result = detect_advertisement(request.program_text, request.ad_text)
        return result
    except Exception as e:
        print(f"Error during detection: {e}", file=sys.stderr)
        raise HTTPException(status_code=500, detail=str(e))


@app.get("/health")
async def health() -> dict:
    """Health check endpoint."""
    return {
        "status": "ok",
        "service": "similarity-detection",
        "model_loaded": True
    }


def main():
    """Start the FastAPI service."""
    # Read URL from environment and extract host/port
    service_url = os.getenv("SIMILARITY_SERVICE_URL", "http://127.0.0.1:8002/detect")
    
    # Parse URL to get host and port
    from urllib.parse import urlparse
    parsed = urlparse(service_url)
    host = parsed.hostname or "127.0.0.1"
    port = parsed.port or 8002
    
    uvicorn.run(
        app,
        host=host,
        port=port,
        log_level="info",
        access_log=True
    )


if __name__ == "__main__":
    main()
