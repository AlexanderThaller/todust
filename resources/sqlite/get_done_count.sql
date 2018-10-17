SELECT
  count(*)
FROM
  v1_entries
WHERE
  finished IS NOT null
AND
  project_name LIKE ?1;
