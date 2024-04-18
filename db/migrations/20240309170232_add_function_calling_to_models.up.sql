-- Copyright 2024 StarfleetAI
-- SPDX-License-Identifier: Apache-2.0

ALTER TABLE models ADD COLUMN function_calling BOOLEAN NOT NULL DEFAULT FALSE;
CREATE INDEX index_models_on_function_calling ON models (company_id, function_calling DESC);
