-- Copyright 2024 StarfleetAI
-- SPDX-License-Identifier: Apache-2.0

CREATE TABLE messages (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    company_id uuid NOT NULL REFERENCES companies(id),
    chat_id uuid NOT NULL REFERENCES chats(id),
    agent_id uuid REFERENCES agents(id),
    user_id uuid REFERENCES users(id),
    status TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT,
    prompt_tokens INTEGER,
    completion_tokens INTEGER,
    tool_calls JSON,
    tool_call_id TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE INDEX messages_chat_id_idx ON messages (company_id, chat_id, updated_at DESC);
CREATE INDEX messages_agent_id_idx ON messages (company_id, agent_id);
CREATE INDEX messages_user_id_idx ON messages (company_id, user_id);
