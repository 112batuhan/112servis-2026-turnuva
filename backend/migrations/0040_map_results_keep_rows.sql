-- A pool map deleted after it was played keeps its result rows (and their scores):
-- the row just loses its map, like a hand-awarded result does.
ALTER TABLE match_map_results
    ALTER COLUMN pool_map_id DROP NOT NULL,
    DROP CONSTRAINT match_map_results_pool_map_id_fkey,
    ADD CONSTRAINT match_map_results_pool_map_id_fkey
        FOREIGN KEY (pool_map_id) REFERENCES pool_maps(id) ON DELETE SET NULL;
