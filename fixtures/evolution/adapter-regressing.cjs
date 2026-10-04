const fs = require('node:fs')

const args = Object.fromEntries(process.argv.slice(2).reduce((out, value, index, all) => {
  if (index % 2 === 0) out.push([value.slice(2), all[index + 1]])
  return out
}, []))
const input = fs.readFileSync(args['input-file'], 'utf8')
const task = fs.readFileSync(args['task-file'], 'utf8').trim()
const maintained = args['round-id'] === 'r07' ? input.replace('# Required legacy heading\n', '') : input
fs.writeFileSync(args['output-file'], `${maintained.trim()}\n\n## ${args['round-id']}\n${task}\n`)
