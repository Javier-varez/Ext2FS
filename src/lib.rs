use num::Integer;

#[repr(C)]
struct Ext2SuperBlock {
    s_inodes_count: u32,      /* Inodes count */
    s_blocks_count: u32,      /* Blocks count */
    s_r_blocks_count: u32,    /* Reserved blocks count */
    s_free_blocks_count: u32, /* Free blocks count */
    s_free_inodes_count: u32, /* Free inodes count */
    s_first_data_block: u32,  /* First Data Block */
    s_log_block_size: i32,    /* Block size */
    s_log_frag_size: u32,     /* Fragment size */
    s_blocks_per_group: u32,  /* # Blocks per group */
    s_frags_per_group: u32,   /* # Fragments per group */
    s_inodes_per_group: u32,  /* # Inodes per group */
    s_mtime: u32,             /* Mount time */
    s_wtime: u32,             /* Write time */
    s_mnt_count: u16,         /* Mount count */
    s_max_mnt_count: u16,     /* Maximal mount count */
    s_magic: u16,             /* Magic signature */
    s_state: u16,             /* File system state */
    s_errors: u16,            /* Behaviour when detecting errors */
    s_minor_rev_level: u16,   /* minor revision level */
    s_lastcheck: u32,         /* time of last check */
    s_checkinterval: u32,     /* max. time between checks */
    s_creator_os: u32,        /* OS */
    s_rev_level: u32,         /* Revision level */
    s_def_resuid: u16,        /* Default uid for reserved blocks */
    s_def_resgid: u16,        /* Default gid for reserved blocks */
    /*
     * These fields are for EXT2_DYNAMIC_REV superblocks only.
     *
     * Note: the difference between the compatible feature set and
     * the incompatible feature set is that if there is a bit set
     * in the incompatible feature set that the kernel doesn't
     * know about, it should refuse to mount the filesystem.
     *
     * e2fsck's requirements are more strict; if it doesn't know
     * about a feature in either the compatible or incompatible
     * feature set, it must abort and not try to meddle with
     * things it doesn't understand...
     */
    s_first_ino: u32,              /* First non-reserved inode */
    s_inode_size: u16,             /* size of inode structure */
    s_block_group_nr: u16,         /* block group # of this superblock */
    s_feature_compat: u32,         /* compatible feature set */
    s_feature_incompat: u32,       /* incompatible feature set */
    s_feature_ro_compat: u32,      /* readonly-compatible feature set */
    s_uuid: [u8; 16],              /* 128-bit uuid for volume */
    s_volume_name: [char; 16],     /* volume name */
    s_last_mounted: [char; 64],    /* directory where last mounted */
    s_algorithm_usage_bitmap: u32, /* For compression */
    /*
     * Performance hints.  Directory preallocation should only
     * happen if the EXT2_COMPAT_PREALLOC flag is on.
     */
    s_prealloc_blocks: u8,     /* Nr of blocks to try to preallocate*/
    s_prealloc_dir_blocks: u8, /* Nr to preallocate for dirs */
    s_padding1: u16,
    /*
     * Journaling support valid if EXT3_FEATURE_COMPAT_HAS_JOURNAL set.
     */
    s_journal_uuid: [u8; 16], /* uuid of journal superblock */
    s_journal_inum: u32,      /* inode number of journal file */
    s_journal_dev: u32,       /* device number of journal file */
    s_last_orphan: u32,       /* start of list of inodes to delete */
    s_hash_seed: [u32; 4],    /* HTREE hash seed */
    s_def_hash_version: u8,   /* Default hash version to use */
    s_reserved_char_pad: u8,
    s_reserved_word_pad: u16,
    s_default_mount_opts: u32,
    s_first_meta_bg: u32,   /* First metablock block group */
    s_reserved: [u32; 190], /* Padding to the end of the block */
}

#[derive(Default)]
struct Ext2GroupDescriptor {
    bg_block_bitmap: u32,
    bg_inode_bitmap: u32,
    bg_inode_table: u32,
    bg_free_blocks_count: u16,
    bg_free_inodes_count: u16,
    bg_used_dirs_count: u16,
    bg_flags: u16,
    bg_reserved: u32,
    bg_itable_unused: u16,
    bg_checksum: u16,
}

/// Trait for a block device. It reads/writes in chunks given by the block size
pub trait BlockDevice {
    /// Reads multiple blocks from the device. The size of the returned block can be obtained with
    /// `get_block_size`
    fn read_blocks(&self, index: usize, num_blocks: usize) -> Vec<u8>;

    /// Writes to the device. The size of the block can be obtained with
    /// `get_block_size`
    fn write_blocks(&mut self, index: usize, data: &[u8]);

    /// Returns the block size of the device
    fn get_block_size(&self) -> usize;
}

#[derive(Debug)]
pub enum Error {
    NoFilesystemFound,
}

/// Representation of an ext2 filesystem
pub struct Ext2Fs<T: BlockDevice> {
    device: T,
    superblock: Option<Ext2SuperBlock>,
    cached_group_descriptor: Ext2GroupDescriptor,
    block_size: usize,
    num_block_groups: usize,
}

impl<T: BlockDevice> Ext2Fs<T> {
    const DEFAULT_BLOCK_SIZE: usize = 1024;
    const SUPERBLOCK_OFFSET: usize = 1024;

    /// Constructor for an ext2 filesystem. It takes ownership of the underlying block device.
    pub fn new(device: T) -> Self {
        Ext2Fs {
            device,
            superblock: None,
            cached_group_descriptor: Default::default(),
            block_size: 1024,
            num_block_groups: 0,
        }
    }

    fn read_superblock(&mut self) -> Result<Ext2SuperBlock, Error> {
        let block_size = self.device.get_block_size();

        // The superblock is located at a fixed 1024 byte offset in the disk
        let index = Self::SUPERBLOCK_OFFSET / block_size;
        let offset = Self::SUPERBLOCK_OFFSET % block_size;
        let block_count = if std::mem::size_of::<Ext2SuperBlock>() > (block_size - offset) {
            let remaining_bytes = std::mem::size_of::<Ext2SuperBlock>() - (block_size - offset);
            1 + remaining_bytes.div_ceil(&block_size)
        } else {
            1
        };

        let superblock_data = self.device.read_blocks(index, block_count);
        let superblock_ptr = superblock_data[offset..].as_ptr() as *const Ext2SuperBlock;
        // SAFETY: 1. It is guaranteed that superblock_ptr will contain enough data for the
        //            superblock, since we read enough data.
        //         2. since the superblock is made of primitive types, its state cannot be invalid.
        //         3. Layout is guaranteed, the Ext2SuperBlock struct is declared with repr(C)
        let superblock: Ext2SuperBlock = unsafe { std::mem::transmute_copy(&*superblock_ptr) };

        if superblock.s_magic != 0xEF53 {
            return Err(Error::NoFilesystemFound);
        }

        Ok(superblock)
    }

    pub fn initialize(&mut self) -> Result<(), Error> {
        self.superblock = Some(self.read_superblock()?);
        let superblock = self.superblock.as_ref().unwrap();

        // Get block size
        let log_block_size = superblock.s_log_block_size;
        self.block_size = if log_block_size < 0 {
            Self::DEFAULT_BLOCK_SIZE >> -log_block_size
        } else {
            Self::DEFAULT_BLOCK_SIZE << log_block_size
        };

        // Extract number of block groups
        self.num_block_groups = superblock
            .s_blocks_count
            .div_ceil(&superblock.s_blocks_per_group) as usize;

        Ok(())
    }

    pub fn block_size(&self) -> usize {
        self.block_size
    }

    pub fn num_block_groups(&self) -> usize {
        self.num_block_groups
    }

    pub fn num_blocks(&self) -> usize {
        if let Some(superblock) = self.superblock.as_ref() {
            return superblock.s_blocks_count as usize;
        }
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::prelude::*;

    struct FileDevice {
        data: Vec<u8>,
    }

    impl FileDevice {
        const BLOCK_SIZE: usize = 1024;

        fn new(path: &std::path::Path) -> Self {
            let mut file = std::fs::File::open(path).unwrap();
            let mut dev = FileDevice { data: vec![] };
            file.read_to_end(&mut dev.data).unwrap();
            dev
        }
    }

    impl BlockDevice for FileDevice {
        fn read_blocks(&self, index: usize, num_blocks: usize) -> Vec<u8> {
            self.data
                .iter()
                .cloned()
                .skip(index * FileDevice::BLOCK_SIZE)
                .take(FileDevice::BLOCK_SIZE * num_blocks)
                .collect()
        }

        fn write_blocks(&mut self, _index: usize, _data: &[u8]) {}

        fn get_block_size(&self) -> usize {
            FileDevice::BLOCK_SIZE
        }
    }

    #[test]
    fn read_superblock() {
        let path = std::path::PathBuf::from("ext2fs.bin");
        let dev = FileDevice::new(&path);
        let mut ext2fs = Ext2Fs::new(dev);

        ext2fs.initialize().unwrap();
        let superblock = ext2fs.superblock.as_ref().unwrap();
        assert_eq!(superblock.s_magic, 0xEF53);
        assert_eq!(superblock.s_creator_os, 0);
        assert_eq!(superblock.s_state, 1);
        assert_eq!(superblock.s_blocks_per_group, 32768); // 32768 blocks per group
        assert_eq!(superblock.s_log_block_size, 2); // 4096 bytes/ block
        assert_eq!(superblock.s_block_group_nr, 0);

        assert_eq!(ext2fs.block_size(), 4096);
        assert_eq!(ext2fs.num_block_groups(), 1);
        assert_eq!(ext2fs.num_blocks(), 256);
    }
}
