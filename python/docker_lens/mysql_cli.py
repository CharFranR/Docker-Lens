import os

from .base import DbEngine
from .validators import sanitize_table_name


class MysqlEngine(DbEngine):

    def connect_command(self, port, user, db_name):
        return f"mysql -hlocalhost -P{port} -u{user} -D{db_name}"

    def shell_args(self, host, port, user, password, database):
        args = ["mysql", f"-h{host}", f"-P{port}", f"-u{user}"]
        if database:
            args.append(f"-D{database}")
        return args

    def shell_env(self, password: str):
        env = {**os.environ}
        if password:
            env["MYSQL_PWD"] = password
        return env

    def parse_tables(self, raw):
        return [line.strip() for line in raw.splitlines()
                if line.strip()
                and "Tables_in_" not in line
                and not line.startswith("+")
                and not line.startswith("|")]

    def head(self, table_name, limit):
        return f"SELECT * FROM {sanitize_table_name(table_name)} LIMIT {limit};"

    def tail(self, table_name, limit):
        return f"SELECT * FROM {sanitize_table_name(table_name)} ORDER BY 1 DESC LIMIT {limit};"

    def schema(self, table_name, db_name=""):
        return (
            f"SELECT column_name, data_type, is_nullable, column_default "
            f"FROM information_schema.columns "
            f"WHERE table_name = '{sanitize_table_name(table_name)}' "
            f"AND table_schema = '{sanitize_table_name(db_name)}' "
            f"ORDER BY ordinal_position;"
        )

    def count(self, table_name):
        return f"SELECT COUNT(*) FROM {sanitize_table_name(table_name)};"

    def truncate(self, table_name):
        return f"TRUNCATE TABLE {sanitize_table_name(table_name)};"

    def drop(self, table_name):
        return f"DROP TABLE {sanitize_table_name(table_name)};"
