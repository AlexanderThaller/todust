SELECT
  count(*)
FROM
  v1_entries
WHERE
  project_name LIKE ?1;
