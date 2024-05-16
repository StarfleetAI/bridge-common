-- Copyright 2024 StarfleetAI
-- SPDX-License-Identifier: Apache-2.0

CREATE SEQUENCE agents_id_bigint_seq;

CREATE TABLE agents (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    id_int INTEGER NOT NULL DEFAULT nextval('agents_id_bigint_seq'),
    company_id uuid NOT NULL REFERENCES companies(id),
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    system_message TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE UNIQUE INDEX agents_id_int_idx ON agents (id_int);
CREATE INDEX index_agents_on_company_id ON agents (company_id);

CREATE TABLE agent_abilities (
    company_id uuid NOT NULL REFERENCES companies(id),
    agent_id uuid NOT NULL REFERENCES agents(id),
    ability_id uuid NOT NULL REFERENCES abilities(id),

    PRIMARY KEY (company_id, agent_id, ability_id)
);

CREATE INDEX agent_abilities_agent_id_idx ON agent_abilities (agent_id);
CREATE INDEX agent_abilities_ability_id_idx ON agent_abilities (ability_id);
