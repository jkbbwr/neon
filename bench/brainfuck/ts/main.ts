declare const process: any;

class Op {
    kind: string;
    val: number;
    body: Op[] | null;

    constructor(kind: string, val: number = 0, body: Op[] | null = null) {
        this.kind = kind;
        this.val = val;
        this.body = body;
    }
}

function parse(source: string): Op[] {
    const [ops] = parseBody(source, 0);
    return ops;
}

function parseBody(source: string, pos: number): [Op[], number] {
    const acc: Op[] = [];
    while (pos < source.length) {
        const c = source[pos];
        if (c === '+' || c === '-') {
            let val = (c === '+') ? 1 : -1;
            pos++;
            while (pos < source.length && (source[pos] === '+' || source[pos] === '-')) {
                val += (source[pos] === '+') ? 1 : -1;
                pos++;
            }
            if (val !== 0) acc.push(new Op('add', val));
        } else if (c === '>' || c === '<') {
            let val = (c === '>') ? 1 : -1;
            pos++;
            while (pos < source.length && (source[pos] === '>' || source[pos] === '<')) {
                val += (source[pos] === '>') ? 1 : -1;
                pos++;
            }
            if (val !== 0) acc.push(new Op('move', val));
        } else if (c === '.') {
            acc.push(new Op('out'));
            pos++;
        } else if (c === ',') {
            acc.push(new Op('in'));
            pos++;
        } else if (c === '[') {
            const [body, nextPos] = parseBody(source, pos + 1);
            acc.push(new Op('loop', 0, body));
            pos = nextPos;
        } else if (c === ']') {
            return [acc, pos + 1];
        } else {
            pos++;
        }
    }
    return [acc, pos];
}

interface State {
    ptr: number;
}

function execute(ops: Op[], tape: Int32Array, state: State): void {
    const limit = ops.length;
    for (let i = 0; i < limit; i++) {
        const op = ops[i];
        switch (op.kind) {
            case 'add':
                tape[state.ptr] += op.val;
                break;
            case 'move':
                state.ptr += op.val;
                break;
            case 'out':
                process.stdout.write(tape[state.ptr].toString());
                break;
            case 'in':
                break;
            case 'loop':
                while (tape[state.ptr] !== 0) {
                    execute(op.body!, tape, state);
                }
                break;
        }
    }
}

function main(): void {
    const program = "++++++++++[>++++++++++[>++++++++++[>++++++++++[>++++++++++[>++++++++++[>++++++++++[>++++++++++[>+<-]<-]<-]<-]<-]<-]<-]<-]";
    const ops = parse(program);
    const tape = new Int32Array(30000);
    const state: State = { ptr: 0 };
    execute(ops, tape, state);
    console.log(`Result: ${tape[8]}`);
}

main();
