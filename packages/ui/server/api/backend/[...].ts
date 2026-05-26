export default defineEventHandler(async (event) => {
  const backendUrl = process.env.BACKEND_URL || 'http://localhost:8080';
  const path = event.path.replace(/^\/api\/backend/, '/api');

  const [basePath, queryString] = path.split('?');
  const cleanParams = new URLSearchParams(queryString ?? '');
  for (const [key, value] of cleanParams.entries()) {
    if (!value) {
      cleanParams.delete(key);
    }
  }
  const query = cleanParams.toString();
  const cleanPath = query ? `${basePath}?${query}` : basePath;

  console.log(cleanPath);
  return proxyRequest(event, `${backendUrl}${cleanPath}`);
});
