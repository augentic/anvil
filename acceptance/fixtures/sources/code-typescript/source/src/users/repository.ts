export interface User {
  id: string;
  email: string;
  createdAt: Date;
}

interface NewUser {
  email: string;
  password: string;
}

export async function insertUser(input: NewUser): Promise<User> {
  return {
    id: cryptoRandomId(),
    email: input.email,
    createdAt: new Date(),
  };
}

function cryptoRandomId(): string {
  return Math.random().toString(36).slice(2, 12);
}
