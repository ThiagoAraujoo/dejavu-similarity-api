#!/usr/bin/env python3
"""
Download and cache the SentenceTransformer model locally.
Called during deployment to avoid runtime HuggingFace Hub requests.
"""
import os
import sys
from sentence_transformers import SentenceTransformer


def main() -> None:
    model_name = os.getenv("SIMILARITY_MODEL_NAME", "paraphrase-multilingual-MiniLM-L12-v2")
    model_path = os.getenv("SIMILARITY_MODEL_PATH", "/opt/dejavu/backend/models/sentence-transformer")

    print(f"Model name: {model_name}")
    print(f"Model path: {model_path}")

    if (
        os.path.exists(model_path)
        and os.path.exists(os.path.join(model_path, "config.json"))
        and (
            os.path.exists(os.path.join(model_path, "model.safetensors"))
            or os.path.exists(os.path.join(model_path, "pytorch_model.bin"))
        )
    ):
        print(f"✅ Model already exists at {model_path}, skipping download")
        return

    print(f"⬇️ Downloading {model_name} from HuggingFace Hub...")
    os.makedirs(os.path.dirname(model_path), exist_ok=True)
    model = SentenceTransformer(model_name, device="cpu")
    model.save(model_path)
    print(f"✅ Model saved to {model_path}")


if __name__ == "__main__":
    try:
        main()
    except Exception as e:
        print(f"❌ Model download failed: {e}", file=sys.stderr)
        sys.exit(1)
