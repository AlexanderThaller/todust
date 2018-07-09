UPDATE v1_entries
SET
  project_name = ?1,
  started = ?2,
  finished = ?3,
  text = ?4
WHERE
  uuid = ?5;
