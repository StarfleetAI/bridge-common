-- Copyright 2024 StarfleetAI
-- SPDX-License-Identifier: Apache-2.0

ALTER TABLE chats ADD COLUMN kind TEXT NOT NULL DEFAULT 'Direct';
CREATE INDEX index_chats_on_kind ON chats (company_id, kind);
