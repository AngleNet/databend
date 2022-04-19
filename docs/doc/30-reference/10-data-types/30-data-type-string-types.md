---
title: String
description: Basic String data type.
---

## String Data Types

In Databend, strings can be stored in the VARCHAR field.

| Data Type        | Syntax   |
| -----------------| -------- |
| String           | VARCHAR

## Functions

See [String Functions](/doc/reference/functions/string-functions).


## Example

```sql
CREATE TABLE string_table(text VARCHAR);

DESC string_table;
+-------+--------+------+---------+
| Field | Type   | Null | Default |
+-------+--------+------+---------+
| text  | String | NO   |         |
+-------+--------+------+---------+

INSERT INTO string_table VALUES('databend');

SELECT * FROM string_table;
+----------+
| text     |
+----------+
| databend |
+----------+
```
