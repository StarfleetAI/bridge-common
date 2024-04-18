-- Copyright 2024 StarfleetAI
-- SPDX-License-Identifier: Apache-2.0

CREATE TABLE task_results (
    id SERIAL PRIMARY KEY,
    company_id INTEGER REFERENCES companies(id) NOT NULL,
    agent_id INTEGER REFERENCES agents(id) NOT NULL,
    task_id INTEGER REFERENCES tasks(id) NOT NULL,
    kind TEXT NOT NULL DEFAULT 'Text',
    data TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE INDEX task_results_agent_id_idx ON task_results (company_id, agent_id);
CREATE INDEX task_results_task_id_idx ON task_results (company_id, task_id, created_at);
CREATE INDEX task_results_kind_idx ON task_results (company_id, kind);
