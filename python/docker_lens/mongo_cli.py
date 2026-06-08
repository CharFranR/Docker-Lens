import json
import os

from .base import DbEngine
from .validators import sanitize_table_name


class MongoEngine(DbEngine):

    def connect_command(self, port, user, db_name):
        if user:
            return f"mongosh 'mongodb://{user}@localhost:{port}/{db_name}'"
        return f"mongosh 'mongodb://localhost:{port}/{db_name}'"

    def shell_args(self, host, port, user, password, database):
        if user:
            uri = f"mongodb://{user}:{password}@{host}:{port}/{database}"
        else:
            uri = f"mongodb://{host}:{port}/{database}"
        return ["mongosh", uri]

    def shell_env(self, password: str):
        return os.environ.copy()

    def parse_tables(self, raw):
        return [line.strip() for line in raw.splitlines()
                if line.strip()
                and "No collections" not in line]

    def head(self, table_name, limit):
        return json.dumps({
            "find": sanitize_table_name(table_name),
            "filter": {},
            "limit": limit
        })

    def tail(self, table_name, limit):
        return json.dumps({
            "aggregate": sanitize_table_name(table_name),
            "pipeline": [
                {"$sort": {"_id": -1}},
                {"$limit": limit}
            ]
        })

    def schema(self, table_name, db_name=""):
        return json.dumps({
            "find": sanitize_table_name(table_name),
            "limit": 1
        })

    def count(self, table_name):
        return json.dumps({
            "aggregate": sanitize_table_name(table_name),
            "pipeline": [{"$count": "total"}]
        })

    def truncate(self, table_name):
        # MongoDB doesn't have TRUNCATE — return a message
        return json.dumps({
            "delete": sanitize_table_name(table_name),
            "deletes": [{"q": {}, "limit": 0}]
        })

    def drop(self, table_name):
        return json.dumps({
            "drop": sanitize_table_name(table_name)
        })
