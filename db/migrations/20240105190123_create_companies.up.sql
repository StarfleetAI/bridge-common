-- Copyright 2024 StarfleetAI
-- SPDX-License-Identifier: Apache-2.0

CREATE TABLE companies (
    id SERIAL PRIMARY KEY,
    auth_id TEXT,
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE INDEX index_companies_on_auth_id ON companies (auth_id);
CREATE UNIQUE INDEX index_companies_on_slug ON companies (slug);
