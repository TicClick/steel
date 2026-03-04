from abc import ABC, abstractmethod
from dataclasses import dataclass


@dataclass
class UploadResult:
    original_filename: str
    browser_download_url: str


class Uploader(ABC):
    @abstractmethod
    def upload(self, file_path: str, tag: str) -> UploadResult:
        """Upload file and return its metadata name and public URL."""
        ...
