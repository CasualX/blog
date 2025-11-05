document.addEventListener("DOMContentLoaded", () => {
  const buttons = document.querySelectorAll(".tag-filter-btn");
  const posts = document.querySelectorAll(".post-card");

  buttons.forEach(btn => {
    btn.addEventListener("click", () => {
      // update button active state
      buttons.forEach(b => b.classList.remove("active"));
      btn.classList.add("active");

      const selectedTag = btn.dataset.tag;

      posts.forEach(post => {
        const raw = post.dataset.tags || "";
        // tags are stored as comma-separated values like: "JavaScript, Web Dev"
        const tags = raw
          .split(",")
          .map(s => s.trim())
          .filter(Boolean);

        const match =
          selectedTag === "all" ||
          tags.some(t => t.toLowerCase() === selectedTag.toLowerCase());

        post.style.display = match ? "block" : "none";
      });
    });
  });
});
