import React from 'react';
import {useParams} from 'react-router-dom';
import {get_session, send_finish, send_register, send_start, send_vote} from "./Manager";
import {Card, Table, Row, Col, Button, Spinner, Form} from "react-bootstrap";
import {account} from "./Web3";

class Details extends React.Component {
    constructor(props) {
        super(props);
        const {account} = props;
        console.log('account', account);
        this.state = {
            session: null,
            pending: null,
            decision: 0
        };
    }

    async refresh() {
        const {managerAddress} = this.props;
        const session = await get_session(managerAddress);
        this.setState({session})
    }


    componentDidMount() {
        this.refresh()
    }

    render() {
        const {managerAddress, account} = this.props;
        const {session, pending, decision} = this.state;

        console.log('account', account);

        const handleRegister = async () => {
            if (account) {
                try {
                    this.setState({pending: 'register'})
                    let {sessionKey, accountId, signature} = await account.signRegistration(session.contract, session.votingId, session.managerAddress);
                    const ticket = await send_register(managerAddress, accountId, signature, sessionKey);
                    const {managerPubKey, resolvedAddress} = account.validateTicket(session.contract, session.votingId, accountId, ticket);
                    console.log('isValid', resolvedAddress == managerAddress, 'managerPubKey', managerPubKey, 'a', resolvedAddress, managerAddress);
                    const newSession = await get_session(managerAddress);
                    this.setState({session: newSession});
                }
                finally {
                    this.setState({pending: null})
                }

            }
        };

        const handleStart = async () => {
            if (account) {
                try {
                    this.setState({pending: 'start'});
                    await send_start(session.managerAddress);
                    const newSession = await get_session(managerAddress);
                    this.setState({session: newSession});
                }
                finally {
                    this.setState({pending: null})
                }

            }
        };

        const handleFinish = async () => {
            if (account) {
                try {
                    this.setState({pending: 'finish'});
                    await send_finish(session.managerAddress);
                    const newSession = await get_session(managerAddress);
                    this.setState({session: newSession});
                }
                finally {
                    this.setState({pending: null})
                }

            }
        };

        const handleVote = async () => {
            if (account) {
                try {
                    this.setState({pending: 'vote'});
                    const accountId = account.accountId;
                    const ticket = session.tickets[accountId.slice(2)];
                    const {managerPubKey, resolvedAddress} = account.validateTicket(session.contract, session.votingId, accountId, ticket);
                    if (resolvedAddress != session.managerAddress) {
                        console.error('unable to resolve pubkey')
                    }
                    else {
                        const messageBytes = await account.encryptVote(this.state.decision, ticket, managerPubKey);
                        let result = await account.decryptVote(managerPubKey, messageBytes);
                        console.log('result', result);
                        await send_vote(session.managerAddress, accountId.slice(2), messageBytes);
                    }
                    const newSession = await get_session(managerAddress);
                    this.setState({session: newSession});
                }
                finally {
                    this.setState({pending: null})
                }
            }
        };

        const render_init = (state) => <>
            <tr><th colSpan={2}>In registration stage</th> </tr>
            <tr><th>minVoters</th><td>{state.minVoters}</td></tr>
            <tr><th>registration deadline</th><td>{state.registrationDeadline}</td></tr>
            <tr><th>Voters</th><td><ul>{state.voters.map((voter) => <li>{voter}</li>)}</ul></td></tr>
            </>;

        const render_voting = (state) => <>
            <tr><th colSpan={2}>In voting stage</th> </tr>
            <tr><th>voting deadline</th><td>{state.votingDeadline}</td></tr>
            <tr><th><i className="oi oi-people"></i> Voters</th><td><ul>{state.voters.map((voter) => <li>{voter.address} -- {voter.voted ? 'Y' : <i className="oi oi-pencil"></i>}</li>)}</ul></td></tr>
        </>;

        const render_report = (state, tickets) => <>
            <tr><th colSpan={2}>Report</th> </tr>
            <tr><th>abstain</th><td>{state.votes['0'] || 0}</td></tr>
            <tr><th>yea</th><td>{state.votes['1'] || 0}</td></tr>
            <tr><th>no</th><td>{state.votes['2'] || 0}</td></tr>
            <tr><th>signature</th><td>
                <code>{state.signature.slice(0, 64)}
                </code> <code>{state.signature.slice(64, 128)}
            </code> <code>{state.signature.slice(128)}</code>
            </td></tr>
        </>;


        const buttonClass = pending == null ? 'primary' : 'disabled';

        const icon = (name) => name === pending ? <Spinner animation="grow" size="sm"/> : '';

        const registered = account && session && account.accountId.substring(2) in session.tickets;

        const hasVoted = (s) => {
            if (account) {
                const accountId = account.accountId.slice(2);
                let myStatus = s.voters.find((v) => account.accountId.substring(2) == v.address);
                return myStatus.voted;
            }
            return false
        };

        const isVotingFinished = (s) => !s.voters.find((v) => !v.voted);

        const info = (session) => <Card border={true}>
            <Card.Header>
                <Card.Title>Current Status {account ? "" : "!"}</Card.Title>
            </Card.Header>
            <Card.Body>
               <Table bordered striped>
                   <tbody>
                        <tr><th>Address</th><td>{managerAddress}</td></tr>
                        <tr><th>Contract</th><td>{session.contract}</td></tr>
                        <tr><th>Voting Id</th><td>{session.votingId}</td></tr>
                        { session.state.init ? render_init(session.state.init) : "" }
                        { session.state.voting ? render_voting(session.state.voting) : "" }
                        { session.state.report ? render_report(session.state.report, session.tickets) : "" }
                   </tbody>
               </Table>
            </Card.Body>
            <Card.Footer>
                {session.state.init && !registered ? <Button onClick={handleRegister} variant={buttonClass} disabled={!!pending}>{icon('register')} Register Voter</Button> : "" }&nbsp;
                {session.state.init ? <Button variant="danger" disabled={!!pending} onClick={handleStart}>{icon('start')} Start</Button> : "" }&nbsp;
                {session.state.voting && isVotingFinished(session.state.voting) ? <Button variant="danger" disabled={!!pending} onClick={handleFinish}>{icon('finish')} Finish</Button> : "" }&nbsp;
                {session.state.voting && !hasVoted(session.state.voting) ?
                <Form><Form.Group>
                    <Form.Control as="select" value={decision} onChange={(e) => this.setState({decision: e.target.value})} disabled={!!pending}>
                        <option value={0}>abstain</option>
                        <option value={1}>yea</option>
                        <option value={2}>no</option>
                    </Form.Control>
                </Form.Group>
                    <Button variant={buttonClass} disabled={!!pending} onClick={handleVote}>{icon('vote')} Vote</Button>&nbsp;
                </Form> :"" }
            </Card.Footer>
        </Card>;
        const credentials = (credentials) => <Card>
            <Card.Header>
                <Card.Title>Validation</Card.Title>
                <Card.Subtitle>enclave attestation result</Card.Subtitle>
            </Card.Header>
            <Card.Body>
                <code><pre>{JSON.stringify(credentials.sgx, null, 4)}</pre></code>
            </Card.Body>
        </Card>;

        return <Row>
            <Col lg={5}>{session ? info(session) : ""}</Col>
            <Col lg={7}>{session ? credentials(session.credentials||session.creditioals) : ""}</Col>
        </Row>
    }
}
export default function SessionDetails(props) {
    let { managerAddress } = useParams();
    return <Details managerAddress={managerAddress} account={props.account}/>;
}