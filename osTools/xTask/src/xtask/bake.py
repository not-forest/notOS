''' Baking Post-Compilation Utilities.

    Provides an interface to create raw `.img`/`.iso` bootable files,
    extracting flat binaries from ELF files.
'''

import os
import logging
import subprocess

from typing import Any
from elftools.elf.elffile import ELFFile

from .constants import XTASK_ERROR_MISSING_KEY

logger = logging.getLogger('xTask')

class ImageBaker:
    ''' Image Baker Utility 
        
        Interface to extract flat binaries from output elf and
        convert it into required image format, ready to be put
        to required hardware.
    '''

    def __init__(self, elfs: list[str], config: dict[str, Any]):
        ''' Create a new instance of ImageBaker. 

            Input:
                elfs: A list of elf binaries to be included. Each ELF is flattened to stripped
                binaries via `flat_bin` function as a result.
                config: Parsed xTask configuration YAML in form of a dictionary.
        '''
        for path in elfs:
            if not os.path.exists(path):
                logger.error(f'Provided elf binary path does not exist or incorrect {path}')
                raise FileNotFoundError()

        self.elfs = elfs
        self.target_dir = os.path.dirname(self.elfs[0])

        try:
            self.output_name = config['builder']['name']
            self.objcopy = config['builder']['objcopy']
            self.format = config['builder']['format']
        except KeyError as e:
            logger.error(f'Mandatory field missing in YAML builder: {e}')
            raise XTASK_ERROR_MISSING_KEY

    def bake(self) -> int:
        ''' Bakes ELF files into OS image, based on the selected configuration format.
        '''
        match self.format:
            case 'image':
                return self.image()
            case 'iso':
                raise RuntimeError('Unimplemented: iso')
            case 'binary':
                raise RuntimeError('Unimplemented: binary')
        
        return 0
    
    def get_elf_phys_addr(self, elf_path: str) -> int:
        ''' Parses elf file and gets starting physical address.
        '''
        with open(elf_path, 'rb') as f:
            return ELFFile(f).header['e_entry']

    def flat_bin(self):
        ''' Generates a flat binary from elf.
        
            Selected objcopy utility depends on the architecture and selected from YAML
            configuration.
        '''
        if self.objcopy == None:
            logger.error('Mandatory objcopy field not supplied in YAML profile.')
            raise XTASK_ERROR_MISSING_KEY

        for elf in self.elfs:
            logger.info(f'Creating flat binary for: {os.path.basename(elf)}')
            cmd = [self.objcopy, '-O', 'binary', elf, elf + '.bin'] 
            subprocess.run(cmd, check=True)

    def image(self) -> int:
        ''' Concatenates all provided elf files into a single OS image. 

            From all binaries, selects the lowest address as a starting position. Each ELF
            is expected to be a static kernel module (e.g. bootloader or kernel itself). It
            is completely forbidden to provide user-space elf files mixed with kernel modules,
            since they are operating on virtual addresses.

            Returns: The size of output image.
        '''
        if not self.elfs:
            logger.error('No ELF binaries provided to build an image.')
            raise RuntimeError('Image: No ELF binaries provided. Make sure YAML contains "recipe" field.')

        elf_memory_map = {}
        for elf in self.elfs:
            phys_addr = self.get_elf_phys_addr(elf)
            elf_memory_map[elf] = phys_addr
            logger.debug(f'Physical load address for {os.path.basename(elf)}: {hex(phys_addr)}')

        # The lowest physical address becomes byte 0 of the disk image
        file_base_memory = min(elf_memory_map.values())
        logger.info(f'Universal image baseline reference: {hex(file_base_memory)}')
        self.flat_bin()
        final_image_path = os.path.join(self.target_dir, self.output_name)

        logger.info(f'Initializing empty image target: {final_image_path}')
        with open(final_image_path, 'wb') as f_out:
            f_out.write(b'')

        # Patching the layout segments sequentially via relative offsets
        with open(final_image_path, 'r+b') as f_out:
            for elf in self.elfs:
                bin_path = elf + '.bin'
                
                with open(bin_path, 'rb') as f_src:
                    raw_bytes = f_src.read()

                # Calculate offset on disk relative to our baseline zero-marker
                elf_phys = elf_memory_map[elf]
                calculated_file_offset = elf_phys - file_base_memory

                logger.info(
                    f'Mapping [{os.path.basename(elf)}] -> Disk File Offset: {hex(calculated_file_offset)} '
                    f'({len(raw_bytes)} bytes written)'
                )
                
                # Seek and bake each kernel module :).
                f_out.seek(calculated_file_offset)
                f_out.write(raw_bytes)

        final_size = os.path.getsize(final_image_path)
        logger.info(f'Image successfully baked ({final_size} bytes) -> {final_image_path}')
        
        return final_size
