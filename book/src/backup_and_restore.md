# Backup and Restore

With any Identity Management (IDM) software, it's important you have the capability to restore in case of a disaster -
be that physical damage or a mistake. Kanidm supports backup and restore of the database with multiple methods.

It is important that you only attempt to restore data with the same version of the server that the backup originated
from.

## Method 1 - Automatic Backup

Automatic backups can be generated online by a `kanidmd server` instance by including the `[online_backup]` section in
the `server.toml`. This allows you to run regular backups, defined by a cron schedule, and maintain the number of backup
versions to keep. An example is located in
[examples/server.toml](https://github.com/kanidm/kanidm/blob/master/examples/server.toml).

### S3-Compatible Storage Backup

Kanidm supports backing up to S3-compatible object storage services (AWS S3, MinIO, Ceph, GCS, Azure Blob via S3 API).
To enable S3 backup, add the `[online_backup.s3]` section to your `server.toml`:

```toml
[online_backup]
path = "/var/lib/kanidm/backups/"
schedule = "00 22 * * *"
versions = 7
compression = "gzip"

[online_backup.s3]
bucket = "kanidm-backups"
region = "us-east-1"
# Optional: Custom endpoint for MinIO or other S3-compatible services
# endpoint = "https://minio.example.com"
# Optional: Path prefix for organizing backups
# path_prefix = "production"
# Optional: Storage class (STANDARD, GLACIER, etc.)
# storage_class = "STANDARD"

# For static credentials (not recommended for production)
[online_backup.s3.credentials]
access_key_id = "your-access-key"
secret_access_key = "your-secret-key"

# For IAM role authentication (recommended for EC2/EKS), omit credentials section

# Optional: Server-side encryption
[online_backup.s3.server_side_encryption]
# algorithm = "aws:kms"  # or "AES256"
# kms_key_id = "arn:aws:kms:us-east-1:123456789:key/..."
```

#### Authentication Methods

1. **Static Credentials**: Configure `access_key_id` and `secret_access_key` directly (suitable for testing)
2. **IAM Role Authentication**: Omit the `credentials` section - the server will use IAM roles when running on EC2/EKS
3. **Environment Variables**: Set `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, and optionally `AWS_SESSION_TOKEN`

#### Backup Verification

Each S3 backup includes a SHA-256 checksum that is verified automatically on restore. You can verify backups
without restoring using:

```bash
kanidmd database verify-s3 -c /data/server.toml backup-2024-01-01T22:00:00Z.json.gz
```

## Method 2 - Manual Backup

This method uses the same process as the automatic process, but is manually invoked. This can be useful for pre-upgrade
backups

To take the backup (assuming our docker environment) you first need to stop the instance:

```bash
docker stop <container name>
docker run --rm -i -t -v kanidmd:/data -v kanidmd_backups:/backup \
    kanidm/server:latest /sbin/kanidmd database backup -c /data/server.toml \
    /backup/kanidm.backup.json
docker start <container name>
```

You can then restart your instance. DO NOT modify the backup.json as it may introduce data errors into your instance.

To restore from the backup:

```bash
docker stop <container name>
docker run --rm -i -t -v kanidmd:/data -v kanidmd_backups:/backup \
    kanidm/server:latest /sbin/kanidmd database restore -c /data/server.toml \
    /backup/kanidm.backup.json
docker start <container name>
```

### Restoring from S3

To restore from an S3 backup:

```bash
docker stop <container name>
docker run --rm -i -t -v kanidmd:/data \
    kanidm/server:latest /sbin/kanidmd database restore-s3 -c /data/server.toml \
    --bucket kanidm-backups --key backup-2024-01-01T22:00:00Z.json.gz
docker start <container name>
```

## Method 3 - Manual Database Copy

This is a simple backup of the data volume containing the database files. Ensure you copy the whole folder, rather than
individual files in the volume!

```bash
docker stop <container name>
# Backup your docker's volume folder
# cp -a /path/to/my/volume /path/to/my/backup-volume
docker start <container name>
```

Restoration is the reverse process where you copy the entire folder back into place.
