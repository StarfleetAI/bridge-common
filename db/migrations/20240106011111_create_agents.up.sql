-- Copyright 2024 StarfleetAI
-- SPDX-License-Identifier: Apache-2.0

CREATE TABLE agents (
    id SERIAL PRIMARY KEY,
    company_id INTEGER REFERENCES companies(id) NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    system_message TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL
);

CREATE INDEX index_agents_on_company_id ON agents (company_id);

CREATE TABLE agent_abilities (
    company_id INTEGER REFERENCES companies(id) NOT NULL,
    agent_id INTEGER REFERENCES agents(id) NOT NULL,
    ability_id INTEGER REFERENCES abilities(id) NOT NULL,

    PRIMARY KEY (company_id, agent_id, ability_id)
);

CREATE INDEX agent_abilities_agent_id_idx ON agent_abilities (agent_id);
CREATE INDEX agent_abilities_ability_id_idx ON agent_abilities (ability_id);
