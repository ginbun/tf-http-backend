FROM ubuntu:22.04

RUN apt-get update -y
RUN apt-get install -y python3 python3-pip python3-dev default-libmysqlclient-dev build-essential pkg-config

WORKDIR /app

COPY ./requirements.txt /app/requirements.txt
COPY ./.flaskenv /app/.flaskenv
COPY ./app.py /app/app.py
COPY ./db_init.sql /app/db_init.sql

RUN python3 -m pip install setuptools pip --upgrade
RUN python3 -m pip install -r requirements.txt
CMD ["python3", "app.py", "--config", "config.ini"]