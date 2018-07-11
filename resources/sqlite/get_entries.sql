SELECT
  project_name,
  started,
  finished,
  uuid,
  text
FROM
  v1_entries
WHERE
  project_name IS ?1;
