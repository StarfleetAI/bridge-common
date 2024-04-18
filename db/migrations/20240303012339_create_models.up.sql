-- Copyright 2024 StarfleetAI
-- SPDX-License-Identifier: Apache-2.0

CREATE TABLE models (
    id SERIAL PRIMARY KEY,
    company_id INTEGER REFERENCES companies(id) NOT NULL,
    provider TEXT NOT NULL DEFAULT '',
    name TEXT NOT NULL DEFAULT '',
    context_length INTEGER NOT NULL,
    max_tokens INTEGER NOT NULL,
    text_in BOOLEAN NOT NULL DEFAULT false,
    text_out BOOLEAN NOT NULL DEFAULT false,
    image_in BOOLEAN NOT NULL DEFAULT false,
    image_out BOOLEAN NOT NULL DEFAULT false,
    audio_in BOOLEAN NOT NULL DEFAULT false,
    audio_out BOOLEAN NOT NULL DEFAULT false,
    api_url TEXT,
    api_key TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE UNIQUE INDEX index_models_on_cpn ON models (company_id, provider, name);
