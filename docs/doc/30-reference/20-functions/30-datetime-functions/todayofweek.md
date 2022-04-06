---
title: toDayOfWeek
---

Converts a date or date with time to a UInt8 number containing the number of the day of the week (Monday is 1, and Sunday is 7).

## Syntax

```sql
toDayOfWeek(expr)
```

## Arguments

| Arguments   | Description |
| ----------- | ----------- |
| expr | date16/date32/datetime |

## Return Type
`UInt8` datatype.

## Examples

```sql
mysql> select toDayOfWeek(toDate(18869));
+----------------------------+
| toDayOfWeek(toDate(18869)) |
+----------------------------+
|                          1 |
+----------------------------+

mysql> select toDayOfWeek(now());
+--------------------+
| toDayOfWeek(now()) |
+--------------------+
|                  2 |
+--------------------+
```
