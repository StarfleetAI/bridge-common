-- Copyright 2024 StarfleetAI
-- SPDX-License-Identifier: Apache-2.0

CREATE TABLE chats (
    id SERIAL PRIMARY KEY,
    company_id INTEGER REFERENCES companies(id) NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE INDEX index_chats_on_updated_at ON chats (company_id, updated_at DESC);

CREATE TABLE agents_chats (
    company_id INTEGER REFERENCES companies(id) NOT NULL,
    agent_id INTEGER REFERENCES agents(id) NOT NULL,
    chat_id INTEGER REFERENCES chats(id) NOT NULL,

    PRIMARY KEY (company_id, agent_id, chat_id)
);

CREATE INDEX agents_chats_chat_id_idx ON agents_chats (company_id, chat_id);
