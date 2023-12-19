#!python3

from flask import Flask

app = Flask(__name__)

@app.route("/")
def Welcome():
  return "This is my first web app! Yay!"

@app.route("/potato")
def potato():
  return "This is a potato app!"


app.run()