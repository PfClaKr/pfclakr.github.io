import fs from "fs";
import path from "path";
import matter from "gray-matter";

const postsDir = "./www/posts";
const files = fs.readdirSync(postsDir).filter(f => f.endsWith(".md"));

let posts = [];
for (let file of files) {
  const raw = fs.readFileSync(path.join(postsDir, file), "utf8");
  const { content } = matter(raw);
  posts.push({
    title: file.replace(".md", ""),
    date: new Date().toISOString().split("T")[0],
    content // Markdown 원문 그대로 저장
  });
}

fs.writeFileSync(path.join(postsDir, "index.json"), JSON.stringify(posts, null, 2));
