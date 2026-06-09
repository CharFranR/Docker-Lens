from .base import DbEngine
from .validators import sanitize_table_name


class SqliteEngine(DbEngine):

    def connect_command(self, port, user, db_name):
        return f"sqlite3 {db_name}"

    def shell_args(self, host, port, user, password, database):
        return ["sqlite3", database]

    def shell_env(self, password: str):
        import os
        return os.environ.copy()

    def parse_tables(self, raw):
        return [line.strip() for line in raw.splitlines() if line.strip() and "No tables" not in line]

    def head(self, table_name, limit):
        return f"SELECT * FROM {sanitize_table_name(table_name)} LIMIT {limit};"

    def tail(self, table_name, limit):
        return f"SELECT * FROM {sanitize_table_name(table_name)} ORDER BY 1 DESC LIMIT {limit};"

    def schema(self, table_name, db_name=""):
        return f'PRAGMA table_info("{sanitize_table_name(table_name)}");'

    def count(self, table_name):
        return f"SELECT COUNT(*) FROM {sanitize_table_name(table_name)};"

    def truncate(self, table_name):
        return f"DELETE FROM {sanitize_table_name(table_name)};"

    def drop(self, table_name):
        return f"DROP TABLE {sanitize_table_name(table_name)};"
