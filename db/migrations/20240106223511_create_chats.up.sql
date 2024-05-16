-- Copyright 2024 StarfleetAI
-- SPDX-License-Identifier: Apache-2.0

CREATE TABLE chats (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    company_id uuid NOT NULL REFERENCES companies(id),
    title TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE INDEX index_chats_on_updated_at ON chats (company_id, updated_at DESC);

CREATE TABLE agents_chats (
    company_id uuid NOT NULL REFERENCES companies(id),
    agent_id uuid NOT NULL REFERENCES agents(id),
    chat_id uuid NOT NULL REFERENCES chats(id),

    PRIMARY KEY (company_id, agent_id, chat_id)
);

CREATE INDEX agents_chats_chat_id_idx ON agents_chats (company_id, chat_id);
