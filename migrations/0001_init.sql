CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE IF NOT EXISTS cache_entries (
    id          UUID PRIMARY KEY,
    query_hash  TEXT NOT NULL,
    redis_key   TEXT NOT NULL,
    embedding   vector(1536) NOT NULL,
    created_at  TIMESTAMP NOT NULL DEFAULT NOW()
);

-- HNSW index for approximate nearest-neighbor cosine search
CREATE INDEX IF NOT EXISTS cache_entries_embedding_hnsw
    ON cache_entries USING hnsw (embedding vector_cosine_ops)
    WITH (m = 16, ef_construction = 64);

CREATE INDEX IF NOT EXISTS cache_entries_query_hash_idx
    ON cache_entries (query_hash);
