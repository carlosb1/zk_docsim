from sentence_transformers import SentenceTransformer
import numpy as np
import json

model = SentenceTransformer('all-MiniLM-L6-v2')

def to_fixed_precision(arr, scale=1000):
    return [int(x * scale) for x in arr]

def save_embedding(text, filename):
    emb = model.encode(text)
    fixed = to_fixed_precision(emb)
    with open(filename, 'w') as f:
        json.dump(fixed, f)

# Ejemplo de uso:
save_embedding("El texto que quieres verificar", "embeddings/doc1.json")
save_embedding("Texto de referencia", "embeddings/doc2.json")