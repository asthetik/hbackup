use crate::error::{HbackupError, Result};
use crate::model::job::{ArchiveFormat, Level};
use crate::pipeline::stage::{ScannedFile, Scanner};
use bzip2::Compression as BzCompression;
use bzip2::write::BzEncoder;
use flate2::Compression as GzCompression;
use flate2::write::GzEncoder;
use sevenz_rust2::encoder_options::Lzma2Options;
use sevenz_rust2::{ArchiveEntry, ArchiveWriter};
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;
use xz2::write::XzEncoder;
use zstd::stream::write::Encoder as ZstdEncoder;

pub struct ArchiveExecutor {
    source: PathBuf,
    target: PathBuf,
    ignore: Vec<String>,
}

impl ArchiveExecutor {
    pub fn new(source: PathBuf, target: PathBuf, ignore: Vec<String>) -> Self {
        Self {
            source,
            target,
            ignore,
        }
    }

    pub fn run(&self, format: ArchiveFormat, level: Level) -> Result<()> {
        let files = Scanner::new(self.source.clone(), self.ignore.clone()).scan()?;

        let base_name = self
            .source
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("backup");

        let ext = format.extension();
        let final_path = self.target.join(format!("{}.{}", base_name, ext));
        let tmp_path = self.target.with_extension("tmp");
        let file = File::create(&tmp_path)?;

        let archiver: Box<dyn Archiver> = match format {
            ArchiveFormat::Zip => Box::new(ZipArchiver),
            ArchiveFormat::Tar => Box::new(TarArchiver(RawTarBackend)),
            ArchiveFormat::Sevenz => Box::new(SevenZArchiver),
            ArchiveFormat::Zstd => Box::new(TarArchiver(ZstdBackend)),
            ArchiveFormat::Xz => Box::new(TarArchiver(XzBackend)),
            ArchiveFormat::Gzip => Box::new(TarArchiver(GzipBackend)),
            ArchiveFormat::Bzip2 => Box::new(TarArchiver(Bzip2Backend)),
            ArchiveFormat::Lz4 => Box::new(TarArchiver(Lz4Backend)),
        };

        archiver.archive(file, files, level)?;
        std::fs::rename(tmp_path, final_path)?;

        Ok(())
    }
}

pub trait CompressionBackend {
    type Writer: Write;

    fn build_writer(&self, target: File, level: Level) -> Result<Self::Writer>;

    fn finish_writer(&self, mut writer: Self::Writer) -> Result<()> {
        writer.flush()?;
        Ok(())
    }
}

fn setup_buffered_file(target: File) -> BufWriter<File> {
    BufWriter::with_capacity(128 * 1024, target)
}

pub trait Archiver {
    fn archive(&self, target: File, files: Vec<ScannedFile>, level: Level) -> Result<()>;
}

pub struct TarArchiver<B: CompressionBackend>(pub B);

impl<B: CompressionBackend> Archiver for TarArchiver<B> {
    fn archive(&self, target: File, files: Vec<ScannedFile>, level: Level) -> Result<()> {
        let writer = self.0.build_writer(target, level)?;

        let mut tar = tar::Builder::new(writer);

        for file in files {
            let mut f = File::open(&file.absolute)?;
            tar.append_file(&file.relative, &mut f)?;
        }

        tar.finish()?;
        let inner = tar.into_inner()?;
        self.0.finish_writer(inner)?;
        Ok(())
    }
}

pub struct RawTarBackend;

impl CompressionBackend for RawTarBackend {
    type Writer = fs::File;

    fn build_writer(&self, target: std::fs::File, _level: Level) -> Result<Self::Writer> {
        Ok(target)
    }
}

pub struct ZipArchiver;

impl Archiver for ZipArchiver {
    fn archive(&self, target: File, files: Vec<ScannedFile>, level: Level) -> Result<()> {
        let buf_writer = setup_buffered_file(target);
        let mut zip = zip::ZipWriter::new(buf_writer);

        let level = match level {
            Level::Fastest => 1,
            Level::Faster => 3,
            Level::Default => 6,
            Level::Better => 8,
            Level::Best => 9,
        };
        let options = zip::write::SimpleFileOptions::default().compression_level(Some(level));

        for file in files {
            zip.start_file(file.relative.to_string_lossy(), options)
                .map_err(|e| HbackupError::Archive(format!("Zip start_file error: {}", e)))?;

            let mut f = File::open(&file.absolute)?;
            io::copy(&mut f, &mut zip)?;
        }

        let mut inner_buf_writer = zip
            .finish()
            .map_err(|e| HbackupError::Archive(format!("Zip finish error: {}", e)))?;
        inner_buf_writer.flush()?;
        Ok(())
    }
}

pub struct GzipBackend;

impl CompressionBackend for GzipBackend {
    type Writer = GzEncoder<BufWriter<File>>;

    fn build_writer(&self, target: File, level: Level) -> Result<Self::Writer> {
        let buf_writer = setup_buffered_file(target);

        let level = match level {
            Level::Fastest => GzCompression::fast(),
            Level::Faster => GzCompression::new(3),
            Level::Default => GzCompression::default(),
            Level::Better => GzCompression::new(8),
            Level::Best => GzCompression::best(),
        };

        Ok(GzEncoder::new(buf_writer, level))
    }

    fn finish_writer(&self, writer: Self::Writer) -> Result<()> {
        let mut inner = writer
            .finish()
            .map_err(|e| HbackupError::Archive(format!("Gzip finish error: {}", e)))?;
        inner.flush()?;
        Ok(())
    }
}

pub struct SevenZArchiver;

impl Archiver for SevenZArchiver {
    fn archive(&self, target: File, files: Vec<ScannedFile>, level: Level) -> Result<()> {
        let mut writer = ArchiveWriter::new(target)
            .map_err(|e| HbackupError::Archive(format!("7z init error: {}", e)))?;

        let compression_level = match level {
            Level::Fastest => 1,
            Level::Faster => 3,
            Level::Default => 6,
            Level::Better => 8,
            Level::Best => 9,
        };

        let lzma2_options = Lzma2Options::from_level(compression_level).into();
        writer.set_content_methods(vec![lzma2_options]);

        for file in files {
            let entry_name = file.relative.to_string_lossy().to_string();
            let entry = ArchiveEntry::from_path(&file.absolute, entry_name);
            let file_handle = File::open(&file.absolute)?;

            writer
                .push_archive_entry(entry, Some(file_handle))
                .map_err(|e| HbackupError::Archive(format!("7z push error: {}", e)))?;
        }

        writer
            .finish()
            .map_err(|e| HbackupError::Archive(format!("7z finish error: {}", e)))?;

        Ok(())
    }
}

pub struct ZstdBackend;

impl CompressionBackend for ZstdBackend {
    type Writer = ZstdEncoder<'static, BufWriter<File>>;

    fn build_writer(&self, target: File, level: Level) -> Result<Self::Writer> {
        let buf_writer = setup_buffered_file(target);

        let level = match level {
            Level::Fastest => 1,
            Level::Faster => 3,
            Level::Default => 7,
            Level::Better => 14,
            Level::Best => 22,
        };

        let encoder = ZstdEncoder::new(buf_writer, level)
            .map_err(|e| HbackupError::Archive(format!("Zstd init error: {}", e)))?;

        Ok(encoder)
    }

    fn finish_writer(&self, writer: Self::Writer) -> Result<()> {
        let mut inner = writer
            .finish()
            .map_err(|e| HbackupError::Archive(e.to_string()))?;
        inner.flush()?;
        Ok(())
    }
}

pub struct Bzip2Backend;

impl CompressionBackend for Bzip2Backend {
    type Writer = BzEncoder<BufWriter<File>>;

    fn build_writer(&self, target: File, level: Level) -> Result<Self::Writer> {
        let buf_writer = setup_buffered_file(target);

        let level = match level {
            Level::Fastest => BzCompression::fast(),
            Level::Faster => BzCompression::new(3),
            Level::Default => BzCompression::default(),
            Level::Better => BzCompression::new(8),
            Level::Best => BzCompression::best(),
        };

        Ok(BzEncoder::new(buf_writer, level))
    }

    fn finish_writer(&self, writer: Self::Writer) -> Result<()> {
        let mut inner_writer = writer
            .finish()
            .map_err(|e| HbackupError::Archive(format!("Bzip2 finish error: {}", e)))?;

        inner_writer.flush()?;
        Ok(())
    }
}

pub struct XzBackend;

impl CompressionBackend for XzBackend {
    type Writer = XzEncoder<BufWriter<File>>;

    fn build_writer(&self, target: File, level: Level) -> Result<Self::Writer> {
        let buf_writer = setup_buffered_file(target);

        let xz_level = match level {
            Level::Fastest => 1,
            Level::Faster => 3,
            Level::Default => 6,
            Level::Better => 8,
            Level::Best => 9,
        };

        Ok(XzEncoder::new(buf_writer, xz_level))
    }

    fn finish_writer(&self, writer: Self::Writer) -> Result<()> {
        let mut inner_writer = writer
            .finish()
            .map_err(|e| HbackupError::Archive(format!("Xz finish error: {}", e)))?;

        inner_writer.flush()?;
        Ok(())
    }
}

pub struct Lz4Backend;

impl CompressionBackend for Lz4Backend {
    // 嵌套结构：LZ4 编码器 -> 缓冲区 -> 物理文件
    type Writer = lz4::Encoder<BufWriter<File>>;

    fn build_writer(&self, target: File, level: Level) -> Result<Self::Writer> {
        let buf_writer = setup_buffered_file(target);

        let lz4_level = match level {
            Level::Fastest => 1,
            Level::Faster => 3,
            Level::Default => 6,
            Level::Better => 14,
            Level::Best => 16,
        };

        let encoder = lz4::EncoderBuilder::new()
            .level(lz4_level)
            .build(buf_writer)
            .map_err(|e| HbackupError::Archive(format!("LZ4 init error: {}", e)))?;

        Ok(encoder)
    }

    fn finish_writer(&self, writer: Self::Writer) -> Result<()> {
        let (mut inner_writer, result) = writer.finish();

        result.map_err(|e| HbackupError::Archive(format!("LZ4 finish error: {}", e)))?;

        inner_writer.flush()?;
        Ok(())
    }
}
