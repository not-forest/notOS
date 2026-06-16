''' Logging configuration for xTask
'''

import logging

class xTaskLogFormatter(logging.Formatter):
    def __init__(self):
        super().__init__()
        self.formatters = {
            logging.DEBUG:    logging.Formatter("xTask :) -> [%(levelname)s] %(message)s"),
            logging.INFO:     logging.Formatter("xTask :) -> [%(levelname)s] %(message)s"),
            logging.WARNING:  logging.Formatter("xTask :| -> [%(levelname)s] %(message)s"),
            logging.ERROR:    logging.Formatter("xTask :( -> [%(levelname)s] %(message)s"),
            logging.CRITICAL: logging.Formatter("xTask >:( -> [%(levelname)s] %(message)s"),
        }

    def format(self, record):
        formatter = self.formatters.get(record.levelno, self.formatters[logging.INFO])
        return formatter.format(record)
