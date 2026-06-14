''' xTask Global Constants.
'''

import logging
import os

OS_SUPPORTED_ARCHITECTURES = ['x86']
''' Currently supported architectures. '''

OS_RELATIVE_ROOT_PATH = os.path.normpath(
    os.path.join(os.path.abspath(__file__), '../../..')
)
''' Relative project's root path.
    This does not need any environment variables, since we always expect source files
    of current file to be in a specific place relative to notOS repo root.
'''

XTASK_DEFAULT_PROFILE_PATH = os.path.join(
    OS_RELATIVE_ROOT_PATH, 
    'osTools/xTask/profiles/xtask.default.yaml'
)
''' Default xTask profile YAML. '''

XTASK_LOGGING_LEVELS = {
    'info': logging.INFO, 
    'debug': logging.DEBUG, 
    'none': logging.NOTSET, 
}

''' Logging levels for xTask program. '''
XTASK_COMMANDS = ['build', 'run', 'test']
''' xTask available commands. '''
XTASK_BUILD_ALIAS = ['build', 'run', 'test']
''' xTask commands, which depend on build. '''
