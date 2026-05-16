import { UserService } from '../services/user-service';

export function listUsers(req, res) {
  const users = UserService.getAll();
  res.json(users);
}

export function createUser(req, res) {
  const user = UserService.create(req.body);
  res.status(201).json(user);
}
