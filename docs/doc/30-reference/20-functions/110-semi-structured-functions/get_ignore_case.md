---
title: Get Ignore Case
---

Extracts value from an `OBJECT` by `field_name`, or a `VARIANT` that contains `OBJECT`.
The value is returned as a `Variant` or `NULL` if either of the arguments is `NULL`.

`GET_IGNORE_CASE` is similar to `GET` but applies case-insensitive matching to field names.

## Syntax

```sql
get_ignore_case(object, field_name)
get_ignore_case(variant, field_name)
```

## Arguments

| Arguments   | Description |
| ----------- | ----------- |
| object      | The OBJECT value
| variant     | The VARIANT value that contains either an ARRAY or an OBJECT
| field_name  | The String value specifies the key in a key-value pair of OBJECT

## Return Type

Variant

## Examples

```sql
mysql> select get_ignore_case(parse_json('{"aa":1, "aA":2, "Aa":3}'), 'AA');
+---------------------------------------------------------------+
| get_ignore_case(parse_json('{"aa":1, "aA":2, "Aa":3}'), 'AA') |
+---------------------------------------------------------------+
| 1                                                             |
+---------------------------------------------------------------+
1 row in set (0.01 sec)
```
