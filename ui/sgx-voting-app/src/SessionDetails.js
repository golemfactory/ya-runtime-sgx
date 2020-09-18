import React from 'react';
import {useParams} from 'react-router-dom';
import {get_session} from "./Manager";
import {Card, Table, Row, Col, Button} from "react-bootstrap";
import {account} from "./Web3";

class Details extends React.Component {
    constructor(props) {
        super(props);
        this.state = {
            session: null
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
        const {managerAddress} = this.props;
        const {session} = this.state;

        const handleRegister = () => {
            if (account) {
                account.signRegistration(session.contract, session.votingId, session.managerAddress)
            }
        };

        const render_init = (state) => <>
            <tr><th colSpan={2}>In registration stage</th> </tr>
            <tr><th>minVoters</th><td>{state.minVoters}</td></tr>
            <tr><th>registration deadline</th><td>{state.registrationDeadline}</td></tr>
            <tr><th>Voters</th><td><ul>{state.voters.map((voter) => <li>{voter}</li>)}</ul></td></tr>

            </>;

        const info = (session) => <Card border={true}>
            <Card.Header>
                <Card.Title>Current Status</Card.Title>
            </Card.Header>
            <Card.Body>
               <Table bordered striped>
                   <tbody>
                        <tr><th>Address</th><td>{managerAddress}</td></tr>
                        <tr><th>Contract</th><td>{session.contract}</td></tr>
                        <tr><th>Voting Id</th><td>{session.votingId}</td></tr>
                        { session.state.init ? render_init(session.state.init) : "" }
                   </tbody>
               </Table>
            </Card.Body>
            <Card.Footer>
                <Button onClick={handleRegister}>Register Voter</Button> <Button>Start</Button> <Button>Vote</Button>
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
export default function SessionDetails() {
    let { managerAddress } = useParams();
    return <Details managerAddress={managerAddress}/>;
}