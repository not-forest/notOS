''' General Purpose notOS Build Orchestrator.
    
    Main tool to compile, run, test, debug and troubleshoot
    everything related to notOS source code and output binaries.
'''

import sys
import os
import yaml
import argparse
import logging
import subprocess

from .constants import (
    OS_RELATIVE_ROOT_PATH,
    XTASK_COMMANDS,
    XTASK_BUILD_ALIAS,
    XTASK_DEFAULT_PROFILE_PATH,
    XTASK_LOGGING_LEVELS,
)
from .bake import ImageBaker
from .log import xTaskLogFormatter

logger = logging.getLogger('xTask')

class xTask:
    def __init__(self, profile_path: str):
        ''' Creates a new instance of xTask.
        '''
        self.profile_path = profile_path

        # Always appending the default profile first
        with open(XTASK_DEFAULT_PROFILE_PATH) as default, \
             open(profile_path)                  as profile:
                concat = default.read() + profile.read()
                self.config = yaml.safe_load(concat)
                logger.info(f'Parsing profile: {os.path.basename(profile_path)}')

    def build(self):
        ''' Build the target based on the provided YAML configuration.

            Runs provided recipes and post-build image burning.
        '''
        os.chdir(OS_RELATIVE_ROOT_PATH)
        elfs = []
        tmp_dir = os.path.join(OS_RELATIVE_ROOT_PATH, 'target', 'xTask')

        for i, recipe in enumerate(self.config['builder']['recipes']):
            flags = recipe['flags']
            package = recipe['package']

            logger.info(f'Running recipe no: {i}\n'
                f'Package: {package}\n'
                f'Flags: {flags}' if flags else ''
            )

            cmd = ['cargo', 'clippy', 
                   '--package', package,
                   '-Z', 'unstable-options']
            if flags:
                cmd.extend(flags)
            # Firstly we just run clippy check.
            subprocess.run(cmd, check=True)

            # Then we run the build itself.
            cmd[1] = 'build'
            cmd.extend(['--artifact-dir', tmp_dir])
            subprocess.run(cmd, check=True)

            elfs.append(os.path.join(tmp_dir, package))

        baker = ImageBaker(elfs, self.config)
        bytes_baked = baker.bake()

        logger.info(f'Created OS image {os.path.basename(baker.output_name)}. '
            f'Baked a total of {bytes_baked} bytes.'
        )

    def run(self):
        ''' Run target based on the provided YAML configuration.
        '''
        logger.error('Unimplemented')
        raise RuntimeError

    def test(self):
        ''' Executes target's tester based on provided YAML configuration
        '''
        logger.error('Unimplemented')
        raise RuntimeError

def main():
    parser = argparse.ArgumentParser(
        description='General Purpose notOS Build Orchestrator'
    )
    subparsers = parser.add_subparsers(dest='command', required=True)

    # Appending commands.
    for cmd in XTASK_COMMANDS:
        cmd_parser = subparsers.add_parser(cmd)
        cmd_parser.add_argument(
            '-p', '--profile',
            type=str,
            required=True,
            help=f'Path to xTask build profile YAML to perform the {cmd} step.'
        )

    # Log level can be set manually via this flag.
    parser.add_argument(
        '-ll', '--log-level',
        type=str,
        required=False,
        choices=XTASK_LOGGING_LEVELS,
        help=f'Logging level for xTask. Supported values are {XTASK_LOGGING_LEVELS.keys()}'
    )
    args = parser.parse_args()

    # Logging level setup and smileys.
    ll = XTASK_LOGGING_LEVELS.get(args.log_level, logging.INFO)
    logger.setLevel(ll)
    lhandler = logging.StreamHandler(sys.stdout)
    lhandler.setLevel(ll)
    lhandler.setFormatter(xTaskLogFormatter())
    logger.handlers = [lhandler]

    try:
        xtask = xTask(args.profile)

        if args.command in XTASK_BUILD_ALIAS:
            xtask.build()

        if args.command == 'run':
            xtask.run()

        if args.command == 'test':
            xtask.test()

    except Exception as e:
        logger.error(f'xTask top level execution failed: {e}')
        sys.exit(1)

    logger.info('xTask finished with success.')

if __name__ == '__main__':
    main()
