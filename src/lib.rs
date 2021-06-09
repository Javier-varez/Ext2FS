#[repr(C)]
struct Ext2SuperBlock {
    s_inodes_count: u32,      /* Inodes count */
    s_blocks_count: u32,      /* Blocks count */
    s_r_blocks_count: u32,    /* Reserved blocks count */
    s_free_blocks_count: u32, /* Free blocks count */
    s_free_inodes_count: u32, /* Free inodes count */
    s_first_data_block: u32,  /* First Data Block */
    s_log_block_size: u32,    /* Block size */
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

/// Trait for a block device. It reads/writes in chunks given by the block size
pub trait BlockDevice {
    /// Reads a block from the device. The size of the returned block can be obtained with
    /// `get_block_size`
    fn read_block(&self, index: usize) -> Vec<u8>;

    /// Writes a block to the device. The size of the block can be obtained with
    /// `get_block_size`
    fn write_block(&mut self, index: usize, data: &[u8]);

    /// Returns the block size of the device
    fn get_block_size(&self) -> usize;
}

/// Representation of an ext2 filesystem
pub struct Ext2Fs<T: BlockDevice> {
    device: T,
}

impl<T: BlockDevice> Ext2Fs<T> {
    /// Constructor for an ext2 filesystem. It takes ownership of the underlying block device.
    pub fn new(device: T) -> Self {
        Ext2Fs { device }
    }

    /// Reads the superblock and returns it to the user.
    fn read_superblock(&mut self) -> Ext2SuperBlock {
        // The superblock is always located at a 1024 byte offset in the disk
        let superblock_index = 1024 / self.device.get_block_size();

        let block = self.device.read_block(superblock_index);
        let ext_ptr = block.as_ptr() as *const Ext2SuperBlock;
        let superblock: std::mem::MaybeUninit<Ext2SuperBlock> =
            unsafe { std::mem::transmute_copy(&*ext_ptr) };
        unsafe { superblock.assume_init() }
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
        fn new(path: &std::path::Path) -> Self {
            let mut file = std::fs::File::open(path).unwrap();
            let mut dev = FileDevice { data: vec![] };
            file.read_to_end(&mut dev.data).unwrap();
            dev
        }
    }

    impl BlockDevice for FileDevice {
        fn read_block(&self, index: usize) -> Vec<u8> {
            self.data
                .iter()
                .cloned()
                .skip(index * 1024)
                .take(1024)
                .collect()
        }

        fn write_block(&mut self, _index: usize, _data: &[u8]) {}

        fn get_block_size(&self) -> usize {
            1024
        }
    }

    #[test]
    fn read_superblock() {
        // let path = std::path::PathBuf::from("/home/javier/Documents/code/ext2fs/ext2fs/ext2fs.bin");
        let path = std::path::PathBuf::from("ext2fs.bin");
        let dev = FileDevice::new(&path);
        let mut ext2fs = Ext2Fs::new(dev);

        let superblock = ext2fs.read_superblock();
        assert_eq!(superblock.s_magic, 0xEF53);
        assert_eq!(superblock.s_creator_os, 0);
        assert_eq!(superblock.s_state, 1);
    }
}
