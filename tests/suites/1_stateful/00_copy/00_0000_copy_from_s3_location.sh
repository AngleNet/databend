#!/usr/bin/env bash

CURDIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
. "$CURDIR"/../../../shell_env.sh

echo "drop table if exists ontime200;" | $MYSQL_CLIENT_CONNECT
## Create table
cat $CURDIR/../ontime/create_table.sql | sed 's/ontime/ontime200/g' | $MYSQL_CLIENT_CONNECT

## Copy from s3.
echo "Test copy from file"
echo "copy into ontime200 from 's3://testbucket/admin/data/ontime_200.csv' credentials=(aws_key_id='minioadmin' aws_secret_key='minioadmin') FILE_FORMAT = (type = 'CSV' field_delimiter = ','  record_delimiter = '\n' skip_header = 1)" | $MYSQL_CLIENT_CONNECT

## Result.
echo "select count(1), avg(Year), sum(DayOfWeek)  from ontime200" | $MYSQL_CLIENT_CONNECT

# Truncate the ontime table.
echo "truncate table ontime200" | $MYSQL_CLIENT_CONNECT


## Copy from s3 with files.
echo "copy into ontime200 from 's3://testbucket/admin/data/' credentials=(aws_key_id='minioadmin' aws_secret_key='minioadmin') FILES = ('ontime_200.csv', 'ontime_200_v1.csv') FILE_FORMAT = (type = 'CSV' field_delimiter = ','  record_delimiter = '\n' skip_header = 1)" | $MYSQL_CLIENT_CONNECT
## Result.
echo "select count(1), avg(Year), sum(DayOfWeek)  from ontime200" | $MYSQL_CLIENT_CONNECT
echo "truncate table ontime200" | $MYSQL_CLIENT_CONNECT


## Copy from s3 by directory with pattern.
echo "copy into ontime200 from 's3://testbucket/admin/data/' credentials=(aws_key_id='minioadmin' aws_secret_key='minioadmin') PATTERN = 'ontime.*csv' FILE_FORMAT = (type = 'CSV' field_delimiter = ','  record_delimiter = '\n' skip_header = 1)" | $MYSQL_CLIENT_CONNECT
## Result.
echo "select count(1), avg(Year), sum(DayOfWeek)  from ontime200" | $MYSQL_CLIENT_CONNECT
echo "truncate table ontime200" | $MYSQL_CLIENT_CONNECT


## Copy from parquet
echo "copy into ontime200 from 's3://testbucket/admin/data/' credentials=(aws_key_id='minioadmin' aws_secret_key='minioadmin') PATTERN = 'ontime.*parquet' FILE_FORMAT = (type = 'PARQUET')" | $MYSQL_CLIENT_CONNECT
echo "select count(1), avg(Year), sum(DayOfWeek)  from ontime200" | $MYSQL_CLIENT_CONNECT
echo "truncate table ontime200" | $MYSQL_CLIENT_CONNECT
echo "drop table if exists ontime200;" | $MYSQL_CLIENT_CONNECT


