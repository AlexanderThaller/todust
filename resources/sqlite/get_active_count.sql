SELECT
  count(*)
FROM
  v1_entries
WHERE
  finished IS null
AND
  project_name LIKE ?1;
