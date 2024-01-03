from argparse import ArgumentParser
from configparser import ConfigParser
from flask import Flask, request, jsonify
from werkzeug.exceptions import HTTPException, Locked
from flask_sqlalchemy import SQLAlchemy
from datetime import datetime
import logging
import os

### LOGGING ###
# Format and Get root logger
LOG_LEVEL = logging.DEBUG
logging.basicConfig(
    format='[%(asctime)s][%(levelname)-8s][%(name)-16s][%(lineno)d] %(message)s',
    level=LOG_LEVEL,
    datefmt='%Y-%m-%d %H:%M:%S'
)
logger = logging.getLogger()

### ARGUMENTS ###
parser = ArgumentParser()
parser.add_argument("-c","--config-file", type=str, dest="config_file", default="./config.ini", help="Path to config file.")
args = parser.parse_args()

### GENERAL FUNCTIONS ###
def load_config_file(path: str):
    config = ConfigParser()
    config.read(
        filenames = path
    )
    return config

### APP DEFINITION ###
app = Flask(__name__)

### DATABASE ###
# Read config file
if os.path.exists(args.config_file):
    config = load_config_file(path=args.config_file)
else:
    errored = False
    env_list = ["TFSTATES_DB_HOST","TFSTATES_DB_PORT","TFSTATES_DB_SCHEMA","TFSTATES_DB_USER","TFSTATES_DB_PSWD","TFSTATES_FLASK_DEBUG","TFSTATES_FLASK_LISTEN_ADDRESS","TFSTATES_FLASK_PORT"]
    for item in env_list:
        if item not in os.environ.keys():
            logger.critical(f"Please make sure to set the environment variable {item} if you're not using the --config-file option to run the API")
            errored = True
    if errored:
        exit(1)

    config = {
        "database": {
            "host": os.environ["TFSTATES_DB_HOST"],
            "port": os.environ["TFSTATES_DB_PORT"],
            "schema": os.environ["TFSTATES_DB_SCHEMA"],
            "user": os.environ["TFSTATES_DB_USER"],
            "pswd": os.environ["TFSTATES_DB_PSWD"]
        },
        "flask": {
            "debug": os.environ["TFSTATES_FLASK_DEBUG"],
            "address": os.environ["TFSTATES_FLASK_LISTEN_ADDRESS"],
            "port": os.environ["TFSTATES_FLASK_PORT"]
        }
    }

# Connect to DB
# logger.debug(f"mysql+mysqlconnector://{config['database']['user']}:{config['database']['pswd']}@{config['database']['host']}:{config['database']['port']}/{config['database']['schema']}")
app.config["SQLALCHEMY_DATABASE_URI"] = f"mysql+mysqlconnector://{config['database']['user']}:{config['database']['pswd']}@{config['database']['host']}:{config['database']['port']}/{config['database']['schema']}"
app.config['PROPAGATE_EXCEPTIONS'] = True

db = SQLAlchemy(app=app)


class TFState(db.Model):
    __tablename__ = 'tfstates'
    id = db.Column(db.Integer, primary_key = True)
    name = db.Column(db.String(50), unique = True, nullable = False)
    contents = db.Column(db.JSON, nullable = False)
    created_at = db.Column(db.TIMESTAMP, default = datetime.now(), nullable = False)
    last_changed = db.Column(db.TIMESTAMP, nullable = False)

    def __repr__(self) -> str:
        return f"{self.name - self.contents}"

class LockedState(db.Model):
    __tablename__ = 'locked_tfstates'
    name = db.Column(db.String(50), primary_key = True)

    def __repr__(self) -> str:
        return (self.name)
    
### EXCEPTIONS ###

@app.errorhandler(Exception)
def handle_exception(e):
    # pass through HTTP errors
    if isinstance(e, HTTPException):
        return e
    # now you're handling non-HTTP exceptions only
    res = {'code': 500,
           'errorType': 'Internal Server Error',
           'errorMessage': "Something went really wrong!"}
    res['errorMessage'] = e.message if hasattr(e, 'message') else f'{e}'
    return jsonify(res), 500


### API ###

### API Paths ###
@app.route("/tfstates/all", methods=["GET"])
def get_all_states():
    logger.debug("Querying states")
    states = db.session.query(TFState).all()
    logger.debug("Building output")
    output = []
    logger.debug("Filling Output list")
    for state in states:
        output.append({"id": state.id, "name": state.name, "contents": state.contents, "created_at": state.created_at, "last_changed": state.last_changed})
    logger.debug("Returning output and 200 http code")
    return (output,200)

@app.route("/tfstates/<name>", methods=["GET"])
def get_tfstate(name):
    logger.debug("Building query")
    query = db.session.query(TFState).filter_by(name = name)
    logger.debug("Executing query")
    state = query.one_or_none()
    logger.debug("Checking results")
    if state is not None:
        return state.contents
    else:
        return ("Not Found",404)

@app.route("/tfstates/<name>", methods=["POST"])
def update_tfstate(name):
    query = db.session.query(TFState).filter_by(name = name)
    state = query.one_or_none()
    if state is None:
        logger.debug("Creating TFState ojbect")
        state = TFState()
        state.name         = name
        state.contents     = request.json
        state.created_at   = datetime.now()
        state.last_changed = datetime.now()
        logger.debug(f"TFState object created: \n{state.contents}")
        db.session.add(state)
        logger.debug(f"Object added to database")
        db.session.commit()
        logger.debug(f"Transaction commited to DB")
        http_code = 201
    else:
        logger.debug(f"Got TFState ojbect from DB - ID: {state.id}")
        state.contents = request.json
        logger.debug("Updated contents of TFState")
        state.last_changed = datetime.now()
        logger.debug("Updated last_changed of TFState")
        db.session.commit()
        http_code = 200
    return ("OK", http_code)

@app.route("/tfstates/<name>", methods=["LOCK"])
def lock_tfstate(name):
    logger.debug("Trying to find the state first")
    locked_state = db.session.query(LockedState).get(name)
    if locked_state is None:
        locked_state = LockedState()
        locked_state.name = name
        db.session.add(locked_state)
        db.session.commit()
        return ("State locked", 200)
    else:
        return ("State already locked", 423)

@app.route("/tfstates/<name>", methods=["UNLOCK"])
def unlock_tfstate(name):    
    locked_state = db.session.query(LockedState).get(name)
    if locked_state is not None:
        db.session.delete(locked_state)
        db.session.commit()
        return ("State unlocked", 200)
    else:
        return ("State already unlocked", 409)

### MAIN ###
if __name__ == "__main__":
    app.run(
        host = config['flask']['address'],
        port = config['flask']['port'],
        debug = config['flask']['debug']
    )