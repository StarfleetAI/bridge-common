-- Copyright 2024 StarfleetAI
-- SPDX-License-Identifier: Apache-2.0

CREATE TABLE tasks (
    id SERIAL PRIMARY KEY,
    company_id INTEGER REFERENCES companies(id) NOT NULL,
    user_id INTEGER REFERENCES users(id) NOT NULL,
    agent_id INTEGER REFERENCES agents(id) NOT NULL,
    origin_chat_id INTEGER REFERENCES chats(id),
    control_chat_id INTEGER REFERENCES chats(id),
    execution_chat_id INTEGER REFERENCES chats(id),
    title TEXT NOT NULL DEFAULT '',
    summary TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL,
    ancestry TEXT,
    ancestry_level INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE INDEX tasks_user_id_idx ON tasks (company_id, user_id);
CREATE INDEX tasks_agent_id_idx ON tasks (company_id, agent_id);
CREATE INDEX tasks_origin_chat_id_idx ON tasks (company_id, origin_chat_id);
CREATE INDEX tasks_control_chat_id_idx ON tasks (company_id, control_chat_id);
CREATE INDEX tasks_execution_chat_id_idx ON tasks (company_id, execution_chat_id);
CREATE INDEX tasks_ancestry_idx ON tasks (company_id, ancestry, ancestry_level);
CREATE INDEX tasks_status_idx ON tasks (company_id, status);
CREATE INDEX tasks_created_at_idx ON tasks (company_id, created_at);
