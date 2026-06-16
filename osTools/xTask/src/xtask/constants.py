''' xTask Global Constants.
'''

import logging
import os

OS_SUPPORTED_ARCHITECTURES = ['x86']
''' Currently supported architectures. '''

OS_RELATIVE_ROOT_PATH = os.getenv("NOTOS_ROOT")
''' Project's root path.
    This does not need any environment variables, since we always expect source files
    of current file to be in a specific place relative to notOS repo root.
'''
assert OS_RELATIVE_ROOT_PATH, 'Unable to find notOS root folder. Environment is probably not initialized'

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
