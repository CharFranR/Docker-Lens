from .base import DbEngine
from .validators import sanitize_table_name
import os

class PostgresEngine(DbEngine):

    def connect_command(self, port, user, db_name):
        return f"psql -hlocalhost -p{port} -U{user} -d{db_name}"
    
    def shell_args(self, host, port, user, password, database):
        return ["psql", f"-h{host}", f"-p{port}", f"-U{user}", f"-d{database}"]

    def shell_env(self, password: str):
        return {**os.environ, "PGPASSWORD": password}
    
    def parse_tables (self, raw):
        return [line.split("|")[1].strip() for line in raw.splitlines() if "|" in line and "public" in line]

    def head(self, table_name, limit):
        return f"SELECT * FROM {sanitize_table_name(table_name)} LIMIT {limit};"

    def tail(self, table_name, limit):
        return f"SELECT * FROM {sanitize_table_name(table_name)} ORDER BY id DESC LIMIT {limit};"

    def schema(self, table_name, db_name=""):
        return f"SELECT column_name, data_type, is_nullable, column_default FROM information_schema.columns WHERE table_name = '{sanitize_table_name(table_name)}' ORDER BY ordinal_position;"

    def count(self, table_name):
        return  f"SELECT COUNT(*) FROM {sanitize_table_name(table_name)};"

    def truncate(self, table_name):
        return f"TRUNCATE TABLE {sanitize_table_name(table_name)} CASCADE;"

    def drop(self, table_name):
        return f"DROP TABLE {sanitize_table_name(table_name)} CASCADE;"