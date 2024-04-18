-- Copyright 2024 StarfleetAI
-- SPDX-License-Identifier: Apache-2.0

CREATE TABLE messages (
    id BIGSERIAL PRIMARY KEY,
    company_id INTEGER REFERENCES companies(id) NOT NULL,
    chat_id INTEGER REFERENCES chats(id) NOT NULL,
    agent_id INTEGER REFERENCES agents(id),
    user_id INTEGER REFERENCES users(id),
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
