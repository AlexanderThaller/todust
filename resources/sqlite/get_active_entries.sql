SELECT
  project_name,
  started,
  finished,
  uuid,
  text
FROM
  v1_entries
WHERE
  finished IS null;
