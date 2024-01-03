# Terraform HTTP MySQL Backend

## Pre-requisites

This project is developed using Python 3.10

Install required Linux Libraries

```shell
sudo apt-get install python3-dev default-libmysqlclient-dev build-essential pkg-config
```

```shell
sudo yum install python3-devel mysql-devel pkgconfig
```

Install required Python libraries

```shell
python3 -m pip install -r requirements.txt
```

## How to run the Backend

### Cloning the git repo

```shell
git clone https://gitlab.com/silvarion/python/flask/tf-http-mysql-backend.git
```

Once you have cloned the repo, copy the `config_example.ini` file to a `config.ini` file and fill the parameters with real values.

Alternatively you can create an environment file with the following variables:

```shell
# DATABASE CONFIG
TFSTATES_DB_HOST="somehost.domain"
TFSTATES_DB_PORT=1234
TFSTATES_DB_SCHEMA="someschema"
TFSTATES_DB_USER="someuser"
TFSTATES_DB_PSWD="somepassword"
# FLASK CONFIG
TFSTATES_FLASK_DEBUG="false"
TFSTATES_FLASK_LISTEN_ADDRESS="0.0.0.0"
TFSTATES_FLASK_PORT="5000"

```

```ini
[database]
host    = databasehost.localdomain
port    = 3306
schema  = tf_backend
user    = tf_user
pswd    = super_secret_1234!

[flask]
debug   = True
address = "0.0.0.0"
port    = 8081
```

### Run with Docker

```docker run -d --env-file .env jsanchezd/tf_httpmysql_backend```