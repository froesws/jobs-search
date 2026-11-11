import Button from '@mui/material/Button';
import Divider from '@mui/material/Divider';
import ListItemText from '@mui/material/ListItemText';
import Menu from '@mui/material/Menu';
import MenuItem from '@mui/material/MenuItem';
import Typography from '@mui/material/Typography';
import { open } from '@tauri-apps/plugin-dialog';
import { useState } from 'react';

import { safeMessage } from '../lib/errors';
import { useFeedback } from './FeedbackProvider';
import { useResumeViewModel } from '../viewmodels/useResumeViewModel';

export function ResumeManager() {
  const resumes = useResumeViewModel();
  const feedback = useFeedback();
  const [anchor, setAnchor] = useState<HTMLElement | null>(null);

  const handleUpload = async () => {
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [{ name: 'PDF resume', extensions: ['pdf'] }],
      });
      if (typeof selected !== 'string') return;
      await resumes.upload(selected);
      feedback.notify('Resume processed and profile recalculated.', 'success');
    } catch (error) {
      feedback.notify(safeMessage(error), 'error', true);
    }
  };

  const handleDelete = async (fileId: string, filename: string) => {
    try {
      const profile = await resumes.remove(fileId);
      feedback.notify(
        profile.extras &&
          (profile.extras as { resumes_available?: boolean }).resumes_available === false
          ? 'Last resume removed. The profile is preserved, but searches are blocked until a new resume is uploaded.'
          : `Removed ${filename}; profile recalculated.`,
        'warning',
        true,
      );
    } catch (error) {
      feedback.notify(safeMessage(error), 'error', true);
    }
  };

  return (
    <>
      <Button size="small" variant="outlined" onClick={(event) => setAnchor(event.currentTarget)}>
        Resumes ({resumes.resumes.length})
      </Button>
      <Button
        size="small"
        variant="contained"
        color="secondary"
        onClick={() => void handleUpload()}
        disabled={resumes.busy}
      >
        Upload resume
      </Button>
      <Menu anchorEl={anchor} open={Boolean(anchor)} onClose={() => setAnchor(null)}>
        {resumes.status === 'empty' ? (
          <MenuItem disabled>
            <ListItemText
              primary="No resumes stored"
              secondary="Upload a text-based PDF to build the profile."
            />
          </MenuItem>
        ) : null}
        {resumes.resumes.map((resume) => (
          <MenuItem key={resume.id} sx={{ gap: 2, minWidth: 320 }}>
            <ListItemText
              primary={resume.filename}
              secondary={new Date(resume.uploaded_at).toLocaleString()}
            />
            <Button
              size="small"
              color="error"
              onClick={(event) => {
                event.stopPropagation();
                void handleDelete(resume.id, resume.filename);
              }}
            >
              Delete
            </Button>
          </MenuItem>
        ))}
        {resumes.status === 'error' ? (
          <MenuItem disabled>
            <Typography variant="caption" color="error">
              {resumes.error}
            </Typography>
          </MenuItem>
        ) : null}
        <Divider />
      </Menu>
    </>
  );
}
