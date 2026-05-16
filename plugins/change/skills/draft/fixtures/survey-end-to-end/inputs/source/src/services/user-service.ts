import { db } from '../utils/db';

export class UserService {
  static getAll() {
    return db.query('SELECT * FROM users');
  }

  static create(data) {
    return db.insert('users', data);
  }
}
