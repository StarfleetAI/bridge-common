-- Copyright 2024 StarfleetAI
-- SPDX-License-Identifier: Apache-2.0

CREATE TABLE task_results (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    company_id uuid NOT NULL REFERENCES companies(id),
    agent_id uuid NOT NULL REFERENCES agents(id),
    task_id uuid NOT NULL REFERENCES tasks(id),
    kind TEXT NOT NULL DEFAULT 'Text',
    data TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE INDEX task_results_agent_id_idx ON task_results (company_id, agent_id);
CREATE INDEX task_results_task_id_idx ON task_results (company_id, task_id, created_at);
CREATE INDEX task_results_kind_idx ON task_results (company_id, kind);
