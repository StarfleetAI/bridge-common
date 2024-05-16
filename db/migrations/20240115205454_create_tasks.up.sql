-- Copyright 2024 StarfleetAI
-- SPDX-License-Identifier: Apache-2.0

CREATE TABLE tasks (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    company_id uuid NOT NULL REFERENCES companies(id),
    user_id uuid NOT NULL REFERENCES users(id),
    agent_id uuid NOT NULL REFERENCES agents(id),
    origin_chat_id uuid REFERENCES chats(id),
    control_chat_id uuid REFERENCES chats(id),
    execution_chat_id uuid REFERENCES chats(id),
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
