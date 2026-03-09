with open('src/forensic.rs', 'r') as f:
    content = f.read()

# Make ForensicLogger Cloneable
old_struct = '''pub struct ForensicLogger {
    sender: mpsc::UnboundedSender<ForensicEvent>,
    log_path: PathBuf,
}'''
new_struct = '''#[derive(Clone)]
pub struct ForensicLogger {
    sender: mpsc::UnboundedSender<ForensicEvent>,
    log_path: PathBuf,
}'''
content = content.replace(old_struct, new_struct)

# Fix shutdown signature
old_shutdown = '''pub async fn shutdown(self) -> Result<(), LogError> {'''
new_shutdown = '''pub async fn shutdown(&self) -> Result<(), LogError> {'''
content = content.replace(old_shutdown, new_shutdown)

# Fix shutdown drop sender
old_drop = '''let _ = self.sender.send(session_end);
        drop(self.sender); // Drop sender to close channel and stop task'''
new_drop = '''let _ = self.sender.send(session_end);
        // Note: we can't drop self.sender here because we only have a reference.
        // The channel will close when all clones of ForensicLogger are dropped.'''
content = content.replace(old_drop, new_drop)

with open('src/forensic.rs', 'w') as f:
    f.write(content)
