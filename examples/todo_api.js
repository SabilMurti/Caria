import { serve } from '@hono/node-server';
import { Hono } from 'hono';

import Database from 'better-sqlite3';

const db = await Database(`todos.db`);
async function setup_db() {
    await db.exec(`CREATE TABLE IF NOT EXISTS todos (id INTEGER PRIMARY KEY AUTOINCREMENT, title TEXT NOT NULL, done INTEGER DEFAULT 0)`);
}
const app = new Hono();
app.get('/todos', async (c) => {
    const todos = await db.prepare(`SELECT * FROM todos`).all();
    return await c.json(todos);
});

app.get('/todos/:id', async (c) => {
    const id = await c.req.param(`id`);
    const todo = await db.prepare(`SELECT * FROM todos WHERE id = ?`).all(id);
    return await c.json(todo);
});

app.post('/todos', async (c) => {
    const body = await c.req.json();
    await db.prepare(`INSERT INTO todos (title) VALUES (?)`).run(body.title);
    return await c.text(`created`);
});

app.put('/todos/:id', async (c) => {
    const id = await c.req.param(`id`);
    const body = await c.req.json();
    await db.prepare(`UPDATE todos SET done = ? WHERE id = ?`).run(body.done, id);
    return await c.text(`updated`);
});

app.delete('/todos/:id', async (c) => {
    const id = await c.req.param(`id`);
    await db.prepare(`DELETE FROM todos WHERE id = ?`).run(id);
    return await c.text(`deleted`);
});

async function main() {
    await setup_db();
}

serve({ fetch: app.fetch, port: 3000 });



if (typeof main === 'function') { await main(); }
