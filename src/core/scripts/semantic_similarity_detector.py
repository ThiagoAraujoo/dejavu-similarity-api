#!/usr/bin/env python3
"""
Semantic similarity detector for advertisement matching.
This script is called from Rust to detect if an advertisement appears in a program.
"""

import sys
import json
import os
from sentence_transformers import SentenceTransformer, util
from typing import Tuple, Optional

# Load multilingual model for Portuguese support
MODEL = SentenceTransformer('paraphrase-multilingual-MiniLM-L12-v2')

# Configuration - Read from environment variables with defaults
SIMILARITY_THRESHOLD = float(os.getenv('SIMILARITY_THRESHOLD', '80.0')) / 100.0  # Convert percentage to decimal
WINDOW_SIZE = int(os.getenv('SIMILARITY_WINDOW_SIZE', '50'))  # words per chunk
WINDOW_OVERLAP = int(os.getenv('SIMILARITY_WINDOW_OVERLAP', '25'))  # words overlap between chunks


def split_into_chunks(text: str, window_size: int = WINDOW_SIZE, overlap: int = WINDOW_OVERLAP) -> list:
    """Split text into overlapping chunks of words."""
    words = text.split()
    chunks = []
    
    if len(words) <= window_size:
        return [text]
    
    step = window_size - overlap
    for i in range(0, len(words) - overlap, step):
        chunk_words = words[i:i + window_size]
        chunks.append(' '.join(chunk_words))
    
    return chunks


def find_best_matching_snippet(program_text: str, ad_text: str, ad_embedding) -> Tuple[Optional[str], float]:
    """Find the best matching snippet in the program using sliding window."""
    chunks = split_into_chunks(program_text)
    
    if not chunks:
        return None, 0.0
    
    # Generate embeddings for all chunks
    chunk_embeddings = MODEL.encode(chunks, convert_to_tensor=True)
    
    # Calculate similarity scores
    similarities = util.cos_sim(ad_embedding, chunk_embeddings)[0]
    
    # Find best match
    best_idx = similarities.argmax().item()
    best_score = similarities[best_idx].item()
    best_snippet = chunks[best_idx]
    
    return best_snippet, best_score


def convert_digits_to_number_words(text: str) -> str:
    """Convert digits to Portuguese number words for consistent matching."""
    import re
    
    # Digits to Portuguese number words mapping
    digit_map = {
        '0': 'zero', '1': 'um', '2': 'dois', '3': 'três', '4': 'quatro',
        '5': 'cinco', '6': 'seis', '7': 'sete', '8': 'oito', '9': 'nove',
        '10': 'dez', '11': 'onze', '12': 'doze', '13': 'treze', '14': 'quatorze',
        '15': 'quinze', '16': 'dezesseis', '17': 'dezessete', '18': 'dezoito',
        '19': 'dezenove', '20': 'vinte', '30': 'trinta', '40': 'quarenta',
        '50': 'cinquenta', '60': 'sessenta', '70': 'setenta', '80': 'oitenta',
        '90': 'noventa', '100': 'cem', '200': 'duzentos', '300': 'trezentos',
        '400': 'quatrocentos', '500': 'quinhentos', '600': 'seiscentos',
        '700': 'setecentos', '800': 'oitocentos', '900': 'novecentos',
        '1000': 'mil'
    }
    
    # Convert to lowercase for matching
    text_lower = text.lower()
    
    # Sort by length (longest first) to match larger numbers first (e.g., 1000 before 100)
    sorted_digits = sorted(digit_map.items(), key=lambda x: len(x[0]), reverse=True)
    
    # Replace digits with number words
    for digit, word in sorted_digits:
        # Use word boundaries to avoid partial matches
        text_lower = re.sub(r'\b' + digit + r'\b', word, text_lower)
        
    return text_lower


def normalize_text(text: str) -> str:
    """Normalize text for comparison by removing extra whitespace, lowercasing, and filtering stop words."""
    import re
    
    # First, convert digits to number words
    text = convert_digits_to_number_words(text)
    
    # Common Portuguese stop words to remove
    # Note: 'um' and 'uma' removed from stop words as they can represent the number 1
    stop_words = {
        'a', 'o', 'as', 'os', 'de', 'da', 'do', 'das', 'dos', 'em', 'na', 'no', 'nas', 'nos',
        'para', 'com', 'por', 'que', 'é', 'e', 'uns', 'umas', 'ao', 'aos', 'à', 'às',
        'se', 'ou', 'mas', 'como', 'mais', 'já', 'seu', 'sua', 'seus', 'suas', 'esse', 'essa',
        'esses', 'essas', 'este', 'esta', 'estes', 'estas', 'aquele', 'aquela', 'aqueles', 'aquelas',
        'me', 'te', 'lhe', 'vos', 'lhes', 'meu', 'teu', 'nosso', 'vosso', 'dele', 'dela',
        'deles', 'delas', 'isso', 'isto', 'aquilo', 'ele', 'ela', 'eles', 'elas', 'eu', 'tu',
        'você', 'nós', 'vocês', 'foi', 'ser', 'ter', 'estar', 'há', 'muito', 'muita', 'muitos',
        'muitas', 'pouco', 'pouca', 'poucos', 'poucas', 'todo', 'toda', 'todos', 'todas', 'outro',
        'outra', 'outros', 'outras', 'mesmo', 'mesma', 'mesmos', 'mesmas', 'tal', 'tais', 'qual',
        'quais', 'quanto', 'quanta', 'quantos', 'quantas', 'algum', 'alguma', 'alguns', 'algumas',
        'nenhum', 'nenhuma', 'nenhuns', 'nenhumas', 'cada', 'qualquer', 'quaisquer', 'certo',
        'certa', 'certos', 'certas', 'vário', 'vária', 'vários', 'várias', 'tanto', 'tanta',
        'tantos', 'tantas', 'são', 'eram', 'era', 'está', 'estão', 'tem', 'têm', 'tinha',
        'tinham', 'pode', 'podem', 'pôde', 'puderam', 'vai', 'vão', 'vem', 'vêm', 'veio', 'vieram'
    }
    
    # Remove extra whitespace and normalize
    text = re.sub(r'\s+', ' ', text.strip().lower())
    # Remove punctuation but keep spaces
    text = re.sub(r'[^\w\s]', ' ', text)
    # Remove extra spaces again
    text = re.sub(r'\s+', ' ', text)
    
    # Split into words and filter out stop words
    words = text.split()
    filtered_words = [word for word in words if word not in stop_words and len(word) >= 2]
    
    normalized = ' '.join(filtered_words)
    
    return normalized


def find_longest_common_subsequence(ad_words: list, program_words: list) -> int:
    """
    Find the longest common subsequence (LCS) of words that appear in order.
    This ensures we only match words that appear in the same sequence.
    """
    if not ad_words or not program_words:
        return 0
    
    # Dynamic programming approach for LCS
    m, n = len(ad_words), len(program_words)
    dp = [[0] * (n + 1) for _ in range(m + 1)]
    
    for i in range(1, m + 1):
        for j in range(1, n + 1):
            if ad_words[i-1] == program_words[j-1]:
                dp[i][j] = dp[i-1][j-1] + 1
            else:
                dp[i][j] = max(dp[i-1][j], dp[i][j-1])
    
    return dp[m][n]


def find_sequence_matches(ad_words: list, program_words: list, max_gap: int = 5) -> int:
    """
    Find how many ad words appear in the program in the same order,
    allowing small gaps between words (max_gap words).
    Returns the count of words found in sequence.
    """
    if not ad_words or not program_words:
        return 0
    
    matched_count = 0
    program_idx = 0
    
    for ad_word in ad_words:
        # Search for this word in the remaining program text
        found = False
        search_limit = min(program_idx + max_gap + 50, len(program_words))
        
        for i in range(program_idx, search_limit):
            if program_words[i] == ad_word:
                matched_count += 1
                program_idx = i + 1  # Move past this word
                found = True
                break
        
        # If word not found in reasonable distance, continue searching
        # but don't increment matched_count
        if not found:
            # Try to find it further ahead (allows for missing words)
            for i in range(search_limit, len(program_words)):
                if program_words[i] == ad_word:
                    program_idx = i + 1
                    break
    
    return matched_count


def calculate_word_match_score(ad_words: list, program_words: list) -> tuple:
    """
    Calculate sequence-aware matching score.
    Returns (matched_count, total_ad_words, sequence_ratio, matched_keywords).
    """
    if not ad_words or not program_words:
        return 0, len(ad_words) if ad_words else 0, 0.0, []
    
    # Use sequence matching instead of simple word presence
    sequence_matches = find_sequence_matches(ad_words, program_words, max_gap=5)
    
    # Also calculate LCS for comparison
    lcs_length = find_longest_common_subsequence(ad_words, program_words)
    
    # Use the better of the two methods
    matched_count = max(sequence_matches, lcs_length)
    
    sequence_ratio = matched_count / len(ad_words) if ad_words else 0.0
    
    # Find which keywords actually matched
    program_words_set = set(program_words)
    matched_keywords = [word for word in ad_words if word in program_words_set]
    
    return matched_count, len(ad_words), sequence_ratio, matched_keywords


def detect_advertisement(program_text: str, ad_text: str) -> dict:
    """
    Detect if an advertisement appears in a program transcription using sequence matching.
    Returns a dict with match_found, score, and matched_snippet.
    """
    
    # Normalize texts
    normalized_program = normalize_text(program_text)
    normalized_ad = normalize_text(ad_text)

    print(f"DEBUG: Normalized ad: {normalized_ad}", file=sys.stderr)
    print(f"DEBUG: Normalized program: {normalized_program}", file=sys.stderr)
    
    ad_words = normalized_ad.split()
    program_words = normalized_program.split()
    
    # For very short ads, use semantic similarity
    if len(ad_words) <= 5:
        ad_embedding = MODEL.encode(ad_text, convert_to_tensor=True)

        best_snippet, snippet_similarity = find_best_matching_snippet(program_text, ad_text, ad_embedding)
        
        return {
            "match_found": snippet_similarity >= SIMILARITY_THRESHOLD,
            "score": int(snippet_similarity * 100),
            "matched_snippet": best_snippet if snippet_similarity >= SIMILARITY_THRESHOLD else "",
            "overall_similarity": round(snippet_similarity, 3),
            "chunk_similarity": round(snippet_similarity, 3),
            "ad_keywords": ad_words,
            "matched_keywords": []
        }
    
    # Calculate word match score with sequence awareness
    matched_words, total_ad_words, sequence_ratio, matched_keywords = calculate_word_match_score(ad_words, program_words)
    
    print(f"DEBUG: Matched words: {matched_words}, Total ad words: {total_ad_words}, Sequence ratio: {sequence_ratio}", file=sys.stderr)

    # Use sequence ratio as the final score
    final_score = sequence_ratio
        
    match_found = final_score >= SIMILARITY_THRESHOLD
    
    return {
        "match_found": match_found,
        "score": int(final_score * 100),
        "matched_snippet": ad_text if match_found else "",
        "overall_similarity": round(final_score, 3),
        "chunk_similarity": round(final_score, 3),
        "ad_keywords": ad_words,
        "matched_keywords": matched_keywords
    }


def main():
    """Main function to be called from command line."""
    if len(sys.argv) != 3:
        print(json.dumps({"error": "Usage: semantic_similarity_detector.py <program_text> <ad_text>"}))
        sys.exit(1)
    
    program_text = sys.argv[1]
    ad_text = sys.argv[2]
    
    try:
        result = detect_advertisement(program_text, ad_text)
        print(json.dumps(result))
    except Exception as e:
        print(json.dumps({"error": str(e)}))
        sys.exit(1)


if __name__ == "__main__":
    main()
