import express from "express";
import { registerUser } from "./users/register";

const app = express();
app.use(express.json());
app.post("/users", registerUser);

app.listen(3000);
