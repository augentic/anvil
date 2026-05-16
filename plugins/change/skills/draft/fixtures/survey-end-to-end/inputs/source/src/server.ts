import express from 'express';
import { listUsers, createUser } from './routes/users';

const app = express();

app.get('/health', (req, res) => {
  res.json({ ok: true });
});
app.get('/users', listUsers);
app.post('/users', createUser);

app.listen(3000);
