class DbEngine:
    
    def connect_command(self, port, user, db_name) -> str:
        raise NotImplementedError
    
    def shell_args(self, host, port, user, password, database) -> list[str]:
        raise NotImplementedError

    def shell_env(self, password: str) -> dict:
        raise NotImplementedError
    
    def parse_tables(self, raw) -> list[str]:
        raise NotImplementedError

    def head(self, table_name: str, limit: int) -> str:
        raise NotImplementedError
    
    def tail(self, table_name: str, limit: int) -> str:
        raise NotImplementedError
    
    def schema(self, table_name: str, db_name: str) -> str:
        raise NotImplementedError
    
    def count(self, table_name: str) -> str:
        raise NotImplementedError
    
    def truncate(self, table_name: str) -> str:
        raise NotImplementedError
    
    def drop(self, table_name: str) -> str:
        raise NotImplementedError
    
