const fs = require('node:fs')

const args = Object.fromEntries(process.argv.slice(2).reduce((out, value, index, all) => {
  if (index % 2 === 0) out.push([value.slice(2), all[index + 1]])
  return out
}, []))
const output = fs.readFileSync(args['output-file'], 'utf8')
process.exit(output.includes(`## ${args['round-id']}`) ? 0 : 9)
