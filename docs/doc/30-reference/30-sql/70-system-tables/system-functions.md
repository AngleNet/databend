---
title: system.functions
---

Contains information about scalar, aggregate and user defined functions.

```sql
mysql> SELECT * FROM system.functions limit 10;
```

```text
mysql> SELECT * FROM system.functions limit 10\G;
*************************** 1. row ***************************
        name: today
  is_builtin: 1
is_aggregate: 0
  definition: 
    category: datetime
 description: Returns current date.
      syntax: TODAY()

     example: mysql> select TODAY();
+------------+
| TODAY()    |
+------------+
| 2021-09-03 |
+------------+

*************************** 2. row ***************************
        name: exp
  is_builtin: 1
is_aggregate: 0
  definition: 
    category: numeric
 description: Returns the value of e (the base of natural logarithms) raised to the power of x.
      syntax: EXP(x)

     example: mysql> SELECT EXP(2);
+------------------+
| EXP(2)           |
+------------------+
| 7.38905609893065 |
+------------------+
1 row in set (0.00 sec)

mysql> SELECT EXP(-2);
+--------------------+
| EXP((- 2))         |
+--------------------+
| 0.1353352832366127 |
+--------------------+
1 row in set (0.00 sec)

mysql> SELECT EXP(0);
+--------+
| EXP(0) |
+--------+
|      1 |
+--------+
1 row in set (0.01 sec)

*************************** 3. row ***************************
        name: cos
  is_builtin: 1
is_aggregate: 0
  definition: 
    category: numeric
 description: Returns the cosine of x, where x is given in radians.
      syntax: COS(x)

     example: mysql> SELECT COS(PI());
+-----------+
| COS(PI()) |
+-----------+
|        -1 |
+-----------+
1 row in set (0.00 sec)
Read 1 rows, 1 B in 0.000 sec., 2.64 thousand rows/sec., 2.64 KB/sec.

*************************** 4. row ***************************
        name: touint16
  is_builtin: 1
is_aggregate: 0
  definition: 
    category: 
 description: 
      syntax: 
     example: 
*************************** 5. row ***************************
        name: get_path
  is_builtin: 1
is_aggregate: 0
  definition: 
    category: 
 description: 
      syntax: 
     example: 
*************************** 6. row ***************************
        name: tostartofday
  is_builtin: 1
is_aggregate: 0
  definition: 
    category: datetime
 description: Rounds down a date with time to the start of the day.
      syntax: toStartOfDay(expr)

     example: mysql> select toStartOfDay(now());
+---------------------+
| toStartOfDay(now()) |
+---------------------+
| 2022-03-29 00:00:00 |
+---------------------+

mysql> select toStartOfDay(toDateTime(1630812366));
+--------------------------------------+
| toStartOfDay(toDateTime(1630812366)) |
+--------------------------------------+
| 2021-09-05 00:00:00                  |
+--------------------------------------+

*************************** 7. row ***************************
        name: get_ignore_case
  is_builtin: 1
is_aggregate: 0
  definition: 
    category: 
 description: 
      syntax: 
     example: 
*************************** 8. row ***************************
        name: locate
  is_builtin: 1
is_aggregate: 0
  definition: 
    category: string
 description: Returns 0 if substr is not in str. Returns NULL if any argument is NULL.
      syntax: LOCATE(substr,str)
LOCATE(substr,str,pos)

     example: SELECT LOCATE('bar', 'foobarbar')
+----------------------------+
| LOCATE('bar', 'foobarbar') |
+----------------------------+
|                          4 |
+----------------------------+

SELECT LOCATE('xbar', 'foobar')
+--------------------------+
| LOCATE('xbar', 'foobar') |
+--------------------------+
|                        0 |
+--------------------------+

SELECT LOCATE('bar', 'foobarbar', 5)
+-------------------------------+
| LOCATE('bar', 'foobarbar', 5) |
+-------------------------------+
|                             7 |
+-------------------------------+

*************************** 9. row ***************************
        name: atan
  is_builtin: 1
is_aggregate: 0
  definition: 
    category: numeric
 description: Returns the arc tangent of x, that is, the value whose tangent is x.
      syntax: ATAN(x)

     example: mysql> SELECT ATAN(-2);
+---------------------+
| ATAN((- 2))         |
+---------------------+
| -1.1071487177940906 |
+---------------------+
1 row in set (0.02 sec)

*************************** 10. row ***************************
        name: tosecond
  is_builtin: 1
is_aggregate: 0
  definition: 
    category: datetime
 description: Converts a date with time to a UInt8 number containing the number of the second in the minute (0-59).
      syntax: toSecond(expr)

     example: mysql> select toSecond(now());
+-----------------+
| toSecond(now()) |
+-----------------+
|              14 |
+-----------------+

mysql> select toSecond(toDateTime(1630812366));
+----------------------------------+
| toSecond(toDateTime(1630812366)) |
+----------------------------------+
|                                6 |
+----------------------------------+
```