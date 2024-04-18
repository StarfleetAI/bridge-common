-- Copyright 2024 StarfleetAI
-- SPDX-License-Identifier: Apache-2.0

ALTER TABLE chats ADD COLUMN model_id INTEGER REFERENCES models(id);
CREATE INDEX index_chats_on_model_id ON chats (company_id, model_id);
